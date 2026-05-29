use anyhow::{Context, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use tokio::process::Command;
use tracing::info;
use uuid::Uuid;

#[derive(Deserialize)]
struct ExportEvent {
    project_id: Uuid,
    export_id: Uuid,
}

#[derive(Serialize)]
struct ExportResponse {
    status: String,
    export_id: String,
    output_url: Option<String>,
}

#[derive(sqlx::FromRow)]
struct Clip {
    id: Uuid,
    asset_id: Uuid,
    in_point_ms: i32,
    out_point_ms: i32,
}

async fn handler(event: LambdaEvent<ExportEvent>) -> Result<ExportResponse, Error> {
    let ExportEvent { project_id, export_id } = event.payload;

    info!(%project_id, %export_id, "Lambda export started");

    match run_export(project_id, export_id).await {
        Ok(output_url) => {
            info!(%export_id, "Export completed");
            Ok(ExportResponse {
                status: "completed".to_string(),
                export_id: export_id.to_string(),
                output_url: Some(output_url),
            })
        }
        Err(e) => {
            tracing::error!(%export_id, error = %e, "Export failed");
            Ok(ExportResponse {
                status: "failed".to_string(),
                export_id: export_id.to_string(),
                output_url: None,
            })
        }
    }
}

async fn run_export(project_id: Uuid, export_id: Uuid) -> Result<String> {
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    let db = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .context("Failed to connect to PostgreSQL")?;

    let s3 = build_s3_client().await;

    /* อัปเดตสถานะ */
    sqlx::query("UPDATE export_jobs SET status = 'processing', started_at = NOW() WHERE id = $1")
        .bind(export_id)
        .execute(&db)
        .await?;

    /* ดึง resolution */
    let resolution: String = sqlx::query_scalar(
        "SELECT resolution FROM export_jobs WHERE id = $1"
    )
    .bind(export_id)
    .fetch_one(&db)
    .await
    .unwrap_or_else(|_| "4k".to_string());

    let (width, height, fps, crf, preset) = match resolution.as_str() {
        "4k"   => (3840, 2160, 60, "18", "slow"),
        "720p" => (1280,  720, 30, "23", "veryfast"),
        _      => (1920, 1080, 30, "23", "veryfast"),
    };

    let vf_filter = format!(
        "scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,format=yuv420p,fps={}",
        width, height, width, height, fps
    );

    info!(resolution, width, height, fps, "Export settings");

    /* ดึง clips */
    let clips = sqlx::query_as::<_, Clip>(
        "SELECT c.id, c.asset_id, c.in_point_ms, c.out_point_ms
         FROM clips c
         JOIN tracks t ON c.track_id = t.id
         WHERE c.project_id = $1 AND t.type = 'video' AND c.deleted_at IS NULL
         ORDER BY c.track_position_ms ASC"
    )
    .bind(project_id)
    .fetch_all(&db)
    .await?;

    if clips.is_empty() {
        anyhow::bail!("No clips found for project {}", project_id);
    }

    let temp_dir = format!("/tmp/export_{}", export_id);
    std::fs::create_dir_all(&temp_dir)?;

    let mut segments = Vec::new();

    /* Trim แต่ละ clip */
    for (idx, clip) in clips.iter().enumerate() {
        let asset_key: String = sqlx::query_scalar(
            "SELECT original_url FROM assets WHERE id = $1"
        )
        .bind(clip.asset_id)
        .fetch_one(&db)
        .await?;

        let file_url = get_presigned_url(&s3, &asset_key).await?;
        let segment_path = format!("{}/segment_{}.mp4", temp_dir, idx);

        let start_time = format_ms(clip.in_point_ms);
        let end_time   = format_ms(clip.out_point_ms);

        info!(clip_id = %clip.id, idx, "Trimming segment");

        let status = Command::new("/usr/local/bin/ffmpeg")
            .args([
                "-y",
                "-ss", &start_time,
                "-to", &end_time,
                "-i", &file_url,
                "-c:v", "libx264",
                "-preset", preset,
                "-crf", crf,
                "-vf", &vf_filter,
                "-c:a", "aac",
                "-ar", "44100",
                "-ac", "2",
                &segment_path,
            ])
            .status()
            .await
            .context("Failed to spawn ffmpeg")?;

        if !status.success() {
            anyhow::bail!("ffmpeg trim failed for clip {}", clip.id);
        }

        segments.push(format!("{}/segment_{}.mp4", temp_dir, idx));

        let progress = ((idx + 1) as f32 / clips.len() as f32 * 80.0) as i32;
        sqlx::query("UPDATE export_jobs SET progress_percent = $1 WHERE id = $2")
            .bind(progress)
            .bind(export_id)
            .execute(&db)
            .await?;
    }

    /* Concat */
    info!("Concatenating {} segments", segments.len());
    let concat_list_path = format!("{}/segments.txt", temp_dir);
    let mut concat_file = File::create(&concat_list_path)?;
    for seg in &segments {
        writeln!(concat_file, "file '{}'", seg)?;
    }

    let final_output_path = format!("{}/final_output.mp4", temp_dir);
    let status = Command::new("/usr/local/bin/ffmpeg")
        .args([
            "-y",
            "-f", "concat",
            "-safe", "0",
            "-i", &concat_list_path,
            "-c", "copy",
            &final_output_path,
        ])
        .status()
        .await
        .context("Failed to spawn ffmpeg concat")?;

    if !status.success() {
        anyhow::bail!("ffmpeg concat failed");
    }

    /* Upload */
    info!("Uploading to S3");
    let export_key = format!("exports/{}/{}.mp4", project_id, export_id);
    upload_to_s3(&s3, &final_output_path, &export_key, "video/mp4").await?;

    /* Update DB */
    sqlx::query(
        "UPDATE export_jobs SET status = 'completed', progress_percent = 100,
         output_url = $1, completed_at = NOW() WHERE id = $2"
    )
    .bind(&export_key)
    .bind(export_id)
    .execute(&db)
    .await?;

    let _ = std::fs::remove_dir_all(&temp_dir);

    Ok(export_key)
}

async fn build_s3_client() -> aws_sdk_s3::Client {
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let endpoint_url = std::env::var("AWS_ENDPOINT_URL").ok();
    let access_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();

    let mut builder = aws_sdk_s3::config::Builder::from(
        &aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(aws_sdk_s3::config::Region::new(region))
            .load()
            .await,
    )
    .force_path_style(true);

    if let Some(url) = endpoint_url {
        builder = builder.endpoint_url(url);
    }
    if let (Some(key), Some(secret)) = (access_key, secret_key) {
        builder = builder.credentials_provider(
            aws_sdk_s3::config::Credentials::new(key, secret, None, None, "static"),
        );
    }

    aws_sdk_s3::Client::from_conf(builder.build())
}

async fn get_presigned_url(s3: &aws_sdk_s3::Client, key: &str) -> Result<String> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let req = s3
        .get_object()
        .bucket(bucket)
        .key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(3600))?)
        .await?;
    Ok(req.uri().to_string())
}

async fn upload_to_s3(s3: &aws_sdk_s3::Client, local_path: &str, key: &str, content_type: &str) -> Result<()> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let data = tokio::fs::read(local_path).await
        .with_context(|| format!("Failed to read file: {}", local_path))?;
    let body = aws_sdk_s3::primitives::ByteStream::from(data);
    s3.put_object()
        .bucket(&bucket)
        .key(key)
        .content_type(content_type)
        .body(body)
        .send()
        .await
        .with_context(|| format!("S3 upload failed for key: {}", key))?;
    Ok(())
}

fn format_ms(ms: i32) -> String {
    let total_seconds = ms as f32 / 1000.0;
    let hours = (total_seconds / 3600.0) as i32;
    let minutes = ((total_seconds % 3600.0) / 60.0) as i32;
    let seconds = total_seconds % 60.0;
    format!("{:02}:{:02}:{:06.3}", hours, minutes, seconds)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "export_lambda=info".into()),
        )
        .json()
        .init();

    run(service_fn(handler)).await
}
