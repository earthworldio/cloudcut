use anyhow::{Context, Result};
use aws_sdk_s3::presigning::PresigningConfig;
use shared::models::Clip;
use sqlx::PgPool;
use std::fs::File;
use std::io::Write;
use std::process::Command;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

pub async fn handle_render_export(
    project_id: Uuid,
    export_id: Uuid,
    _idempotency_key: String,
    db: &PgPool,
    s3: &aws_sdk_s3::Client,
) -> Result<()> {
    info!(project_id = %project_id, export_id = %export_id, "🚀 Starting export render");

    /* 1. อัปเดตสถานะเป็น processing */
    sqlx::query("UPDATE export_jobs SET status = 'processing', started_at = NOW() WHERE id = $1")
        .bind(export_id)
        .execute(db)
        .await?;

    /* 2. ดึงข้อมูล Clips จาก Timeline (เฉพาะ Video Track แรกเป็นหลักตามโจทย์) */
    /* เรียงตาม track_position_ms */
    let clips = sqlx::query_as::<_, Clip>(
        "SELECT c.* FROM clips c 
         JOIN tracks t ON c.track_id = t.id 
         WHERE c.project_id = $1 AND t.type = 'video' AND c.deleted_at IS NULL 
         ORDER BY c.track_position_ms ASC"
    )
    .bind(project_id)
    .fetch_all(db)
    .await?;

    if clips.is_empty() {
        warn!(project_id = %project_id, "No clips found for export");
        update_export_status(db, export_id, "failed", Some("No clips found")).await?;
        return Ok(());
    }

    /* 3. เตรียมไดเรกทอรีชั่วคราว */
    let temp_dir = format!("/tmp/export_{}", export_id);
    std::fs::create_dir_all(&temp_dir)?;

    let mut segments = Vec::new();

    /* 4. Trimming Clips */
    for (idx, clip) in clips.iter().enumerate() {
        info!(clip_id = %clip.id, "Trimming segment {}", idx);
        
        /* ดึง Asset Object Key */
        let asset_key: String = sqlx::query_scalar("SELECT original_url FROM assets WHERE id = $1")
            .bind(clip.asset_id)
            .fetch_one(db)
            .await?;

        let file_url = get_presigned_url(s3, &asset_key).await?;
        let segment_path = format!("{}/segment_{}.mp4", temp_dir, idx);
        
        let start_time = format_ms(clip.in_point_ms);
        let end_time = format_ms(clip.out_point_ms);

        let output = Command::new("ffmpeg")
            .args([
                "-y",
                "-ss", &start_time,
                "-to", &end_time,
                "-i", &file_url,
                "-c:v", "libx264",
                "-c:a", "aac",
                "-pix_fmt", "yuv420p", /* เพื่อความเข้ากันได้ */
                &segment_path,
            ])
            .output()
            .context("Failed to execute ffmpeg for trimming")?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            error!(clip_id = %clip.id, error = %err, "ffmpeg trim failed");
            update_export_status(db, export_id, "failed", Some("Trim failed")).await?;
            return Err(anyhow::anyhow!("Trim failed for clip {}", clip.id));
        }

        segments.push(format!("segment_{}.mp4", idx));
        
        /* อัปเดต Progress เบื้องต้น */
        let progress = ((idx + 1) as f32 / clips.len() as f32 * 80.0) as i32;
        sqlx::query("UPDATE export_jobs SET progress_percent = $1 WHERE id = $2")
            .bind(progress)
            .bind(export_id)
            .execute(db)
            .await?;
    }

    /* 5. Concatenation */
    info!("Concatenating segments...");
    let concat_list_path = format!("{}/segments.txt", temp_dir);
    let mut concat_file = File::create(&concat_list_path)?;
    for seg in segments {
        writeln!(concat_file, "file '{}'", seg)?;
    }

    let final_output_path = format!("{}/final_output.mp4", temp_dir);
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-f", "concat",
            "-safe", "0",
            "-i", &concat_list_path,
            "-c", "copy", /* ใช้ copy เพราะ encode มาเหมือนกันหมดแล้วตอน trim */
            &final_output_path,
        ])
        .output()
        .context("Failed to execute ffmpeg for concatenation")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        error!(error = %err, "ffmpeg concat failed");
        update_export_status(db, export_id, "failed", Some("Concat failed")).await?;
        return Err(anyhow::anyhow!("Concat failed"));
    }

    /* 6. Upload to MinIO */
    info!("Uploading final output...");
    let export_key = format!("exports/{}/{}.mp4", project_id, export_id);
    upload_to_minio(s3, &final_output_path, &export_key, "video/mp4").await?;

    /* 7. สร้าง Download URL (แบบ Long-lived 7 วัน) */
    let download_url = get_long_presigned_url(s3, &export_key).await?;

    /* 8. อัปเดตสำเร็จ */
    sqlx::query(
        "UPDATE export_jobs SET 
            status = 'completed', 
            progress_percent = 100, 
            output_url = $1, 
            completed_at = NOW() 
         WHERE id = $2"
    )
    .bind(download_url)
    .bind(export_id)
    .execute(db)
    .await?;

    /* 9. Cleanup */
    let _ = std::fs::remove_dir_all(temp_dir);
    
    info!(export_id = %export_id, "✅ Export completed successfully");
    Ok(())
}

/* Helpers */

fn format_ms(ms: i32) -> String {
    let total_seconds = ms as f32 / 1000.0;
    let hours = (total_seconds / 3600.0) as i32;
    let minutes = ((total_seconds % 3600.0) / 60.0) as i32;
    let seconds = total_seconds % 60.0;
    format!("{:02}:{:02}:{:06.3}", hours, minutes, seconds)
}

async fn get_presigned_url(s3: &aws_sdk_s3::Client, key: &str) -> Result<String> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let req = s3.get_object().bucket(bucket).key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(3600))?).await?;
    Ok(req.uri().to_string())
}

async fn get_long_presigned_url(s3: &aws_sdk_s3::Client, key: &str) -> Result<String> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    /* 7 วัน (Max สำหรับ S3 Presigned URL โดยปกติ) */
    let req = s3.get_object().bucket(bucket).key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(7 * 24 * 3600))?).await?;
    Ok(req.uri().to_string())
}

async fn upload_to_minio(s3: &aws_sdk_s3::Client, local_path: &str, key: &str, content_type: &str) -> Result<()> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let body = aws_sdk_s3::primitives::ByteStream::from_path(local_path).await?;
    s3.put_object().bucket(bucket).key(key).content_type(content_type).body(body).send().await?;
    Ok(())
}

async fn update_export_status(db: &PgPool, export_id: Uuid, status: &str, error_msg: Option<&str>) -> Result<()> {
    sqlx::query("UPDATE export_jobs SET status = $1, error_message = $2, updated_at = NOW() WHERE id = $3")
        .bind(status).bind(error_msg).bind(export_id).execute(db).await?;
    Ok(())
}

use tracing::warn;
