use anyhow::{Context, Result};
use redis::AsyncCommands;
use serde::Deserialize;
use sqlx::PgPool;
use std::process::Command;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;
use aws_sdk_s3::presigning::PresigningConfig;
use shared::models::JobPayload;

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    streams: Vec<StreamInfo>,
    format: FormatInfo,
}

#[derive(Debug, Deserialize)]
struct StreamInfo {
    width: Option<i32>,
    height: Option<i32>,
    r_frame_rate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FormatInfo {
    duration: Option<String>,
}

pub async fn handle_job(
    payload_str: &str,
    db: &PgPool,
    redis: &redis::Client,
    s3: &aws_sdk_s3::Client,
) -> Result<()> {
    let job: JobPayload = serde_json::from_str(payload_str)
        .context("Failed to parse job JSON")?;

    /* 1. Idempotency Check */
    let idempotency_key = match &job {
        JobPayload::ExtractMetadata { idempotency_key, .. } => Some(idempotency_key),
        JobPayload::GenerateProxy { idempotency_key, .. } => Some(idempotency_key),
        JobPayload::GenerateThumbnails { idempotency_key, .. } => Some(idempotency_key),
        JobPayload::ExtractWaveform { idempotency_key, .. } => Some(idempotency_key),
        JobPayload::RenderExport { idempotency_key, .. } => Some(idempotency_key),
        _ => None,
    };

    if let Some(key) = idempotency_key {
        let mut conn = redis.get_multiplexed_async_connection().await?;
        let redis_key = format!("idempotency:{}", key);
        
        /* SETNX with 24 hour TTL */
        let set_res: bool = redis::cmd("SET")
            .arg(&redis_key)
            .arg("processing")
            .arg("NX")
            .arg("EX")
            .arg(86400)
            .query_async(&mut conn)
            .await?;

        if !set_res {
            info!(key = %key, "Job already processed or processing, skipping (idempotency)");
            return Ok(());
        }
    }

    let result = match job.clone() {
        JobPayload::ExtractMetadata { asset_id, input_url, idempotency_key, .. } => {
            handle_extract_metadata(asset_id, input_url, idempotency_key, db, redis, s3).await
        }
        JobPayload::GenerateProxy { asset_id, input_url, idempotency_key, .. } => {
            handle_generate_proxy(asset_id, input_url, idempotency_key, db, s3).await
        }
        JobPayload::GenerateThumbnails { asset_id, input_url, idempotency_key, .. } => {
            handle_generate_thumbnails(asset_id, input_url, idempotency_key, db, s3).await
        }
        JobPayload::ExtractWaveform { asset_id, input_url, idempotency_key, .. } => {
            handle_extract_waveform(asset_id, input_url, idempotency_key, db, s3).await
        }
        JobPayload::RenderExport { project_id, export_id, idempotency_key, .. } => {
            crate::export_pipeline::handle_render_export(project_id, export_id, idempotency_key, db, redis, s3).await
        }
        _ => {
            info!("Unhandled job type or cleanup job");
            Ok(())
        }
    };

    if let Err(e) = result {
        error!(error = %e, "Job execution failed, initiating retry logic");
        handle_retry(job, e.to_string(), redis).await?;
    }

    Ok(())
}

async fn handle_retry(mut job: JobPayload, error_msg: String, redis: &redis::Client) -> Result<()> {
    let max_attempts = 4;
    let current_attempts = match &job {
        JobPayload::ExtractMetadata { attempts, .. } => *attempts,
        JobPayload::GenerateProxy { attempts, .. } => *attempts,
        JobPayload::GenerateThumbnails { attempts, .. } => *attempts,
        JobPayload::ExtractWaveform { attempts, .. } => *attempts,
        JobPayload::RenderExport { attempts, .. } => *attempts,
        JobPayload::CleanupExpiredFiles { .. } => 0, /* No retry for cleanup */
    };

    if let JobPayload::CleanupExpiredFiles { .. } = job {
        return Ok(());
    }

    if current_attempts < max_attempts {
        /* Increment attempts */
        match &mut job {
            JobPayload::ExtractMetadata { attempts, .. } => *attempts += 1,
            JobPayload::GenerateProxy { attempts, .. } => *attempts += 1,
            JobPayload::GenerateThumbnails { attempts, .. } => *attempts += 1,
            JobPayload::ExtractWaveform { attempts, .. } => *attempts += 1,
            JobPayload::RenderExport { attempts, .. } => *attempts += 1,
            JobPayload::CleanupExpiredFiles { .. } => {}
        }

        /* Calculate delay: 2^attempt * 1s */
        let delay_secs = 2u64.pow(current_attempts);
        warn!(attempts = current_attempts + 1, delay = delay_secs, "Retrying job...");

        /* Wait before re-queueing */
        tokio::time::sleep(Duration::from_secs(delay_secs)).await;

        let mut conn = redis.get_multiplexed_async_connection().await?;
        let _: () = conn.lpush("queue:video_pipeline", serde_json::to_string(&job)?).await?;
    } else {
        /* Move to Dead-Letter Queue (DLQ) */
        error!(attempts = current_attempts, "Job failed after max attempts. Moving to DLQ.");
        let dlq_payload = serde_json::json!({
            "job": job,
            "error": error_msg,
            "failed_at": chrono::Utc::now()
        });

        let mut conn = redis.get_multiplexed_async_connection().await?;
        let _: () = conn.lpush("queue:video_pipeline:dead_letter", dlq_payload.to_string()).await?;
    }

    Ok(())
}

async fn handle_extract_metadata(
    asset_id: Uuid,
    input_url: String,
    idempotency_key: String,
    db: &PgPool,
    redis: &redis::Client,
    s3: &aws_sdk_s3::Client,
) -> Result<()> {
    info!(asset_id = %asset_id, "Extracting metadata");

    let file_url = get_presigned_url(s3, &input_url).await?;

    let output = Command::new("ffprobe")
        .args([
            "-v", "error",
            "-select_streams", "v:0",
            "-show_entries", "stream=width,height,r_frame_rate",
            "-show_entries", "format=duration",
            "-of", "json",
            &file_url,
        ])
        .output()
        .context("Failed to execute ffprobe")?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        error!(asset_id = %asset_id, error = %err_msg, "ffprobe failed");
        update_asset_status(db, asset_id, "failed").await?;
        return Err(anyhow::anyhow!("ffprobe failed: {}", err_msg));
    }

    let probe: ProbeOutput = serde_json::from_slice(&output.stdout)?;
    let stream = probe.streams.get(0).context("No video stream found")?;
    let duration_sec: f64 = probe.format.duration.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
    let duration_ms = (duration_sec * 1000.0) as i32;
    let width = stream.width.unwrap_or(0);
    let height = stream.height.unwrap_or(0);
    
    let fps: f64 = if let Some(rate) = &stream.r_frame_rate {
        if let Some((num, den)) = rate.split_once('/') {
            let n: f64 = num.parse().unwrap_or(0.0);
            let d: f64 = den.parse().unwrap_or(1.0);
            if d != 0.0 { n / d } else { 0.0 }
        } else {
            rate.parse().unwrap_or(0.0)
        }
    } else {
        0.0
    };

    let metadata = serde_json::json!({
        "durationMs": duration_ms,
        "width": width,
        "height": height,
        "fps": fps
    });

    sqlx::query("UPDATE assets SET metadata = metadata || $1 WHERE id = $2")
        .bind(metadata)
        .bind(asset_id)
        .execute(db)
        .await?;

    /* แตกงานย่อย: Generate Proxy และ Thumbnails */
    let mut conn = redis.get_multiplexed_async_connection().await?;
    
    let proxy_job = JobPayload::GenerateProxy {
        asset_id,
        input_url: input_url.clone(),
        idempotency_key: format!("{}-proxy", idempotency_key),
        attempts: 0,
    };
    let thumb_job = JobPayload::GenerateThumbnails {
        asset_id,
        input_url: input_url.clone(),
        idempotency_key: format!("{}-thumb", idempotency_key),
        attempts: 0,
    };
    let waveform_job = JobPayload::ExtractWaveform {
        asset_id,
        input_url: input_url.clone(),
        idempotency_key: format!("{}-waveform", idempotency_key),
        attempts: 0,
    };

    let _: () = conn.lpush("queue:video_pipeline", serde_json::to_string(&proxy_job)?).await?;
    let _: () = conn.lpush("queue:video_pipeline", serde_json::to_string(&thumb_job)?).await?;
    let _: () = conn.lpush("queue:video_pipeline", serde_json::to_string(&waveform_job)?).await?;

    info!(asset_id = %asset_id, "Metadata extracted, pushed sub-jobs");
    Ok(())
}

async fn handle_generate_proxy(
    asset_id: Uuid,
    input_url: String,
    _idempotency_key: String,
    db: &PgPool,
    s3: &aws_sdk_s3::Client,
) -> Result<()> {
    info!(asset_id = %asset_id, "Generating proxy");
    let file_url = get_presigned_url(s3, &input_url).await?;
    let temp_output = format!("/tmp/proxy_{}.mp4", asset_id);

    let output = Command::new("ffmpeg")
        .args([
            "-y", "-i", &file_url,
            "-vf", "scale=-2:720",
            "-c:v", "libx264", "-preset", "fast", "-crf", "28",
            "-c:a", "aac", "-b:a", "128k",
            &temp_output,
        ])
        .output()
        .context("Failed to execute ffmpeg for proxy")?;

    if !output.status.success() {
        error!("ffmpeg proxy failed: {}", String::from_utf8_lossy(&output.stderr));
        update_asset_status(db, asset_id, "failed").await?;
        return Err(anyhow::anyhow!("ffmpeg proxy failed"));
    }

    /* Upload Proxy to MinIO */
    let proxy_key = format!("{}_proxy.mp4", input_url);
    upload_to_minio(s3, &temp_output, &proxy_key, "video/mp4").await?;
    
    /* บันทึก variant */
    add_asset_variant(db, asset_id, "proxy", &proxy_key).await?;
    check_and_finalize_asset(db, asset_id).await?;

    let _ = std::fs::remove_file(temp_output);
    Ok(())
}

async fn handle_generate_thumbnails(
    asset_id: Uuid,
    input_url: String,
    _idempotency_key: String,
    db: &PgPool,
    s3: &aws_sdk_s3::Client,
) -> Result<()> {
    info!(asset_id = %asset_id, "Generating thumbnails");
    let file_url = get_presigned_url(s3, &input_url).await?;
    let temp_dir = format!("/tmp/thumbs_{}", asset_id);
    std::fs::create_dir_all(&temp_dir)?;

    let output = Command::new("ffmpeg")
        .args([
            "-y", "-i", &file_url,
            "-vf", "fps=1/5,scale=160:-1",
            "-q:v", "5",
            &format!("{}/thumb_%03d.jpg", temp_dir),
        ])
        .output()
        .context("Failed to execute ffmpeg for thumbnails")?;

    if !output.status.success() {
        error!("ffmpeg thumbnails failed: {}", String::from_utf8_lossy(&output.stderr));
        update_asset_status(db, asset_id, "failed").await?;
        return Err(anyhow::anyhow!("ffmpeg thumbnails failed"));
    }

    /* ในที่นี้เราจะจำลองการเก็บแค่รูปแรกเป็น thumbnail หลัก หรือคุณอาจจะ Zip ก็ได้ */
    /* เพื่อความง่าย เราจะหยิบ thumb_001.jpg ขึ้นไป */
    let thumb_local = format!("{}/thumb_001.jpg", temp_dir);
    let thumb_key = format!("{}_thumb.jpg", input_url);
    
    if std::path::Path::new(&thumb_local).exists() {
        upload_to_minio(s3, &thumb_local, &thumb_key, "image/jpeg").await?;
        add_asset_variant(db, asset_id, "thumbnail", &thumb_key).await?;
    }

    check_and_finalize_asset(db, asset_id).await?;
    let _ = std::fs::remove_dir_all(temp_dir);
    Ok(())
}

async fn handle_extract_waveform(
    asset_id: Uuid,
    input_url: String,
    _idempotency_key: String,
    db: &PgPool,
    s3: &aws_sdk_s3::Client,
) -> Result<()> {
    info!(asset_id = %asset_id, "Extracting waveform");
    let file_url = get_presigned_url(s3, &input_url).await?;
    let temp_raw = format!("/tmp/waveform_{}.raw", asset_id);

    /* 1. Extract raw audio (16-bit signed, mono, 44.1kHz) */
    let output = Command::new("ffmpeg")
        .args([
            "-y", "-i", &file_url,
            "-ac", "1",
            "-ar", "44100",
            "-filter:a", "aformat=sample_fmts=s16",
            "-f", "s16le",
            &temp_raw,
        ])
        .output()
        .context("Failed to execute ffmpeg for waveform")?;

    if !output.status.success() {
        error!("ffmpeg waveform failed: {}", String::from_utf8_lossy(&output.stderr));
        return Err(anyhow::anyhow!("ffmpeg waveform failed"));
    }

    /* 2. Read raw file and extract peaks */
    use std::io::Read;
    let mut file = std::fs::File::open(&temp_raw)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    /* 16-bit = 2 bytes per sample */
    let samples: Vec<i16> = buffer
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();

    /* Downsample to ~500 points */
    let target_points = 500;
    let chunk_size = (samples.len() / target_points).max(1);
    let mut peaks = Vec::new();

    for chunk in samples.chunks(chunk_size) {
        let mut min = 0i16;
        let mut max = 0i16;
        for &sample in chunk {
            if sample < min { min = sample; }
            if sample > max { max = sample; }
        }
        /* Normalize to -1.0 to 1.0 */
        peaks.push(vec![
            min as f32 / 32768.0,
            max as f32 / 32768.0,
        ]);
    }

    let waveform_json = serde_json::json!({
        "sample_rate": 44100,
        "channels": 1,
        "peaks": peaks
    });

    /* 3. Save as AssetVariant (JSON data) */
    /* ในโปรเจกต์นี้เราอาจจะเก็บ JSON ลงใน DB เลยหรือลง S3 */
    /* ตาม Spec 3.5 ให้เก็บเป็น JSON array */
    /* เราจะเก็บลง asset_variants.metadata หรือลง url เป็น json string? */
    /* ตามโมเดล asset_variants มีฟิลด์ metadata: Value */
    
    sqlx::query("INSERT INTO asset_variants (asset_id, type, url, metadata) VALUES ($1, $2, $3, $4)")
        .bind(asset_id)
        .bind("waveform_data")
        .bind("") /* URL ว่างเพราะเก็บใน metadata */
        .bind(waveform_json)
        .execute(db)
        .await?;

    check_and_finalize_asset(db, asset_id).await?;
    let _ = std::fs::remove_file(temp_raw);
    Ok(())
}

/* Helpers */

async fn get_presigned_url(s3: &aws_sdk_s3::Client, key: &str) -> Result<String> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let req = s3.get_object().bucket(bucket).key(key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(3600))?).await?;
    Ok(req.uri().to_string())
}

async fn upload_to_minio(s3: &aws_sdk_s3::Client, local_path: &str, key: &str, content_type: &str) -> Result<()> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let data = tokio::fs::read(local_path).await
        .with_context(|| format!("Failed to read file: {}", local_path))?;
    let body = aws_sdk_s3::primitives::ByteStream::from(data);
    s3.put_object().bucket(bucket).key(key).content_type(content_type).body(body).send().await
        .with_context(|| format!("S3 upload failed for key: {}", key))?;
    Ok(())
}

async fn update_asset_status(db: &PgPool, asset_id: Uuid, status: &str) -> Result<()> {
    sqlx::query("UPDATE assets SET status = $1, updated_at = NOW() WHERE id = $2")
        .bind(status).bind(asset_id).execute(db).await?;
    Ok(())
}

async fn add_asset_variant(db: &PgPool, asset_id: Uuid, kind: &str, url: &str) -> Result<()> {
    sqlx::query("INSERT INTO asset_variants (asset_id, type, url) VALUES ($1, $2, $3)")
        .bind(asset_id).bind(kind).bind(url).execute(db).await?;
    Ok(())
}

async fn check_and_finalize_asset(db: &PgPool, asset_id: Uuid) -> Result<()> {
    /* เช็คว่ามีทั้ง proxy, thumbnail และ waveform_data หรือยัง */
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM asset_variants WHERE asset_id = $1 AND type IN ('proxy', 'thumbnail', 'waveform_data')")
        .bind(asset_id).fetch_one(db).await?;
    
    if count.0 >= 3 {
        update_asset_status(db, asset_id, "ready").await?;
        info!(asset_id = %asset_id, "Asset is now READY");
    }
    Ok(())
}

use tracing::warn;
