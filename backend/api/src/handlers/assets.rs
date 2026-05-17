use axum::{
    extract::{State, Query},
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;
use crate::middleware::auth::Claims;
use crate::error::AppError;
use shared::models::{Asset, PresignedUrlRequest, PresignedUrlResponse, ConfirmUploadRequest};
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;
use crate::handlers::projects::check_project_access;
use redis::AsyncCommands;

/* 1. POST /api/assets/presigned-url */
pub async fn get_presigned_url(
    State(pool): State<PgPool>,
    State(s3): State<S3Client>,
    Claims(user_id): Claims,
    Json(payload): Json<PresignedUrlRequest>,
) -> Result<Json<PresignedUrlResponse>, AppError> {
    /* ตรวจสอบสิทธิ์ Project */
    check_project_access(&pool, payload.project_id, user_id).await?;

    /* สร้าง UUIDv7 สำหรับ Asset ID */
    let asset_id = Uuid::now_v7();
    let bucket_name = std::env::var("S3_BUCKET_NAME").map_err(|_| AppError::Internal(anyhow::Error::msg("S3_BUCKET_NAME not set")))?;
    
    /* ดึง workspace_id เพื่อสร้าง path */
    let row: (Uuid,) = sqlx::query_as("SELECT workspace_id FROM projects WHERE id = $1")
        .bind(payload.project_id)
        .fetch_one(&pool)
        .await?;
    let workspace_id = row.0;

    /* กำหนด Object Key: workspaces/{workspace_id}/projects/{project_id}/{asset_id}_{filename} */
    let object_key = format!(
        "workspaces/{}/projects/{}/{}_{}",
        workspace_id, payload.project_id, asset_id, payload.filename
    );

    /* สร้าง Presigned URL สำหรับ PutObject (หมดอายุใน 15 นาที) */
    let presigned_request = s3
        .put_object()
        .bucket(&bucket_name)
        .key(&object_key)
        .content_type(&payload.content_type)
        .presigned(PresigningConfig::expires_in(Duration::from_secs(900)).map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?)
        .await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(e.to_string())))?;

    Ok(Json(PresignedUrlResponse {
        upload_url: presigned_request.uri().to_string(),
        asset_id,
        object_key,
    }))
}

/* 2. POST /api/assets/confirm-upload */
pub async fn confirm_upload(
    State(pool): State<PgPool>,
    State(redis_client): State<redis::Client>,
    Claims(user_id): Claims,
    Json(payload): Json<ConfirmUploadRequest>,
) -> Result<Json<Asset>, AppError> {
    /* ตรวจสอบสิทธิ์ Project */
    check_project_access(&pool, payload.project_id, user_id).await?;

    /* กำหนด Asset Type จาก Content Type */
    let asset_type = if payload.content_type.starts_with("video/") {
        "video"
    } else if payload.content_type.starts_with("audio/") {
        "audio"
    } else if payload.content_type.starts_with("image/") {
        "image"
    } else {
        "other"
    };

    /* เริ่ม Transaction เพื่อความถูกต้องของข้อมูล */
    let mut tx = pool.begin().await?;

    /* บันทึกลงฐานข้อมูล */
    let asset = sqlx::query_as::<_, Asset>(
        "INSERT INTO assets (id, project_id, uploaded_by, type, original_url, status, metadata) 
         VALUES ($1, $2, $3, $4, $5, $6, $7) 
         RETURNING *"
    )
    .bind(payload.asset_id)
    .bind(payload.project_id)
    .bind(user_id)
    .bind(asset_type)
    .bind(&payload.object_key)
    .bind("processing")
    .bind(serde_json::json!({
        "filename": payload.filename,
        "contentType": payload.content_type,
        "sizeBytes": payload.size_bytes
    }))
    .fetch_one(&mut *tx)
    .await?;

    /* Push job to Redis queue */
    let job_id = Uuid::now_v7();
    let job_payload = serde_json::json!({
        "jobId": job_id,
        "taskType": "extract_metadata",
        "assetId": asset.id,
        "objectKey": asset.original_url,
        "projectId": asset.project_id,
        "workspaceId": (sqlx::query_scalar::<_, Uuid>("SELECT workspace_id FROM projects WHERE id = $1")
            .bind(asset.project_id)
            .fetch_one(&mut *tx)
            .await?)
    });

    let mut conn = redis_client.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis connection error: {}", e))))?;

    let _: () = conn.lpush("queue:video_pipeline", job_payload.to_string()).await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis push error: {}", e))))?;

    /* Commit Transaction */
    tx.commit().await?;

    Ok(Json(asset))
}

/* 3. GET /api/assets */
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListAssetsQuery {
    pub project_id: Uuid,
}

pub async fn list_assets(
    State(pool): State<PgPool>,
    State(s3): State<S3Client>,
    Claims(user_id): Claims,
    Query(query): Query<ListAssetsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    /* ตรวจสอบสิทธิ์ Project */
    check_project_access(&pool, query.project_id, user_id).await?;

    /* ดึงข้อมูล Assets ทั้งหมดที่ยังไม่ถูกลบ */
    let assets = sqlx::query_as::<_, Asset>(
        "SELECT * FROM assets WHERE project_id = $1 AND deleted_at IS NULL ORDER BY created_at DESC"
    )
    .bind(query.project_id)
    .fetch_all(&pool)
    .await?;

    let bucket_name = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    let mut assets_with_urls = Vec::new();

    for asset in assets {
        /* สร้าง Presigned URL สำหรับการอ่าน (GET) - หมดอายุใน 1 ชั่วโมง */
        let presigned_res = s3
            .get_object()
            .bucket(&bucket_name)
            .key(&asset.original_url)
            .presigned(PresigningConfig::expires_in(Duration::from_secs(3600)).unwrap())
            .await;

        let url = match presigned_res {
            Ok(req) => req.uri().to_string(),
            Err(_) => "".to_string(),
        };

        /* แปลงเป็น JSON และแทรก URL เข้าไป */
        let mut asset_json = serde_json::to_value(&asset).unwrap();
        if let Some(obj) = asset_json.as_object_mut() {
            obj.insert("url".to_string(), serde_json::Value::String(url));
        }
        assets_with_urls.push(asset_json);
    }

    Ok(Json(assets_with_urls))
}
