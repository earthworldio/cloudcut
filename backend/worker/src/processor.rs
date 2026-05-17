use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::process::Command;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;
use aws_sdk_s3::presigning::PresigningConfig;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoJob {
    pub job_id: Uuid,
    pub task_type: String,
    pub asset_id: Uuid,
    pub object_key: String,
    pub project_id: Uuid,
    pub workspace_id: Uuid,
}

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

pub async fn process_job(
    job_data: &str,
    db: &PgPool,
    s3_client: &aws_sdk_s3::Client,
) -> Result<()> {
    let job: VideoJob = serde_json::from_str(job_data)
        .context("Failed to parse job JSON")?;

    info!(asset_id = %job.asset_id, "Processing asset");

    /* สร้าง Presigned URL สำหรับ ffprobe อ่านไฟล์ */
    let bucket_name = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let presigned_res = s3_client
        .get_object()
        .bucket(&bucket_name)
        .key(&job.object_key)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(3600))?)
        .await
        .context("Failed to create presigned URL for ffprobe")?;

    let file_url = presigned_res.uri().to_string();

    /*  รัน ffprobe */
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
        .context("Failed to execute ffprobe. Is it installed?")?;

    if !output.status.success() {
        let err_msg = String::from_utf8_lossy(&output.stderr);
        error!(asset_id = %job.asset_id, error = %err_msg, "ffprobe failed");
        
        sqlx::query("UPDATE assets SET status = 'failed' WHERE id = $1")
            .bind(job.asset_id)
            .execute(db)
            .await?;
            
        return Err(anyhow::anyhow!("ffprobe failed: {}", err_msg));
    }

    /* Parse Metadata */
    let probe: ProbeOutput = serde_json::from_slice(&output.stdout)
        .context("Failed to parse ffprobe output")?;

    let stream = probe.streams.get(0).context("No video stream found")?;
    
    let duration_sec: f64 = probe.format.duration.as_deref()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0.0);
    
    let duration_ms = (duration_sec * 1000.0) as i32;
    let width = stream.width.unwrap_or(0);
    let height = stream.height.unwrap_or(0);
    
    /* คำนวณ FPS จาก r_frame_rate (เช่น "30/1" หรือ "24000/1001") */ 
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

    info!(
        asset_id = %job.asset_id,
        duration_ms, width, height, fps,
        "Extraction successful"
    );

    /*  อัปเดต Database */
    let metadata = serde_json::json!({
        "durationMs": duration_ms,
        "width": width,
        "height": height,
        "fps": fps
    });

    sqlx::query("UPDATE assets SET status = 'ready', metadata = metadata || $1, updated_at = NOW() WHERE id = $2")
        .bind(metadata)
        .bind(job.asset_id)
        .execute(db)
        .await?;

    info!(asset_id = %job.asset_id, "Asset status updated to ready");

    Ok(())
}
