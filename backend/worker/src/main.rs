mod processor;

use anyhow::{Context, Result};
use redis::AsyncCommands;
use shared::establish_connection;
use std::time::Duration;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    /* เริ่มต้นระบบ Logging */
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "worker=info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();
    info!("🚀 Worker started, listening for video pipeline jobs...");

    /* เชื่อมต่อ PostgreSQL */
    let db = establish_connection()
        .await
        .context("Failed to connect to PostgreSQL")?;

    /* เชื่อมต่อ Redis */
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url).context("Failed to create Redis client")?;
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await.context("Failed to get Redis connection")?;

    /* ตั้งค่า S3 Client (สำหรับสร้าง Presigned URL ให้ ffprobe) */
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let endpoint_url = std::env::var("AWS_ENDPOINT_URL").ok();
    let access_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
    
    let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(region))
        .load()
        .await)
        .force_path_style(true);
    
    if let Some(url) = endpoint_url {
        s3_config_builder = s3_config_builder.endpoint_url(url);
    }

    if let (Some(key), Some(secret)) = (access_key, secret_key) {
        s3_config_builder = s3_config_builder.credentials_provider(
            aws_sdk_s3::config::Credentials::new(key, secret, None, None, "static")
        );
    }
    
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config_builder.build());

    /* Worker Loop */
    loop {
        /* ใช้ BRPOP เพื่อรอรับงานจาก List (Timeout 0 คือรอไปเรื่อยๆ) */
        /* หมายเหตุ: BRPOP จะคืนค่าเป็น Option<(String, String)> คือ (KeyName, Value) */
        let job_res: Option<(String, String)> = redis_conn
            .brpop("queue:video_pipeline", 0.0)
            .await
            .ok();

        if let Some((_, payload)) = job_res {
            info!("📥 Received new job");
            
            /* ประมวลผลงาน (แยกเป็น Task เพื่อไม่ให้ Block Loop หลัก) */
            let db_clone = db.clone();
            let s3_clone = s3_client.clone();
            
            tokio::spawn(async move {
                if let Err(e) = processor::process_job(&payload, &db_clone, &s3_clone).await {
                    error!(error = %e, "Job processing failed");
                }
            });
        }

        /* พักสักครู่เพื่อป้องกัน CPU ทำงานหนักเกินไปในกรณีที่เกิด Error ต่อเนื่อง */
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
