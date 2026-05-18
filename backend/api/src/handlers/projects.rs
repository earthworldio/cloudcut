use axum::{
    extract::{Path, State, Extension},
    Json,
    response::IntoResponse,
};
use serde_json::json;
use shared::models::{Project, CreateProjectRequest, UpdateProjectRequest, TimelineResponse, TrackWithClips, Track, Clip, JobPayload, AssetVariant};
use redis::AsyncCommands;
use crate::middleware::auth::Claims;
use crate::error::AppError;
use sqlx::{PgPool};
use uuid::Uuid;
use serde::Deserialize;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::presigning::PresigningConfig;
use std::time::Duration;

/* --- Timeline Mutation Handlers --- */

/* 1. เพิ่ม Track ใหม่ */
pub async fn create_track(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>, /* { "type": "video", "label": "New Track" } */
) -> Result<impl IntoResponse, AppError> {
    /* ตรวจสอบสิทธิ์ Project */
    check_project_access(&pool, project_id, user_id).await?;

    let track_id = Uuid::new_v4();
    let t_type = payload["type"].as_str().ok_or(AppError::Validation("Invalid type".into()))?;
    let label = payload["label"].as_str().ok_or(AppError::Validation("Invalid label".into()))?;

    /* หา order_index ล่าสุด */
    let last_order: (i32,) = sqlx::query_as("SELECT COALESCE(MAX(order_index), 0) FROM tracks WHERE project_id = $1")
        .bind(project_id)
        .fetch_one(&pool)
        .await?;

    let track = sqlx::query_as::<_, Track>(
        "INSERT INTO tracks (id, project_id, type, label, order_index) VALUES ($1, $2, $3, $4, $5) RETURNING *"
    )
    .bind(track_id)
    .bind(project_id)
    .bind(t_type)
    .bind(label)
    .bind(last_order.0 + 1)
    .fetch_one(&pool)
    .await?;

    Ok((axum::http::StatusCode::CREATED, Json(track)))
}

/* 2. เพิ่ม Clip ลง Timeline */
pub async fn create_clip(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AppError> {
    check_project_access(&pool, project_id, user_id).await?;

    let clip_id = Uuid::new_v4();
    let track_id = Uuid::parse_str(payload["track_id"].as_str().unwrap()).unwrap();
    let asset_id = Uuid::parse_str(payload["asset_id"].as_str().unwrap()).unwrap();
    
    let clip = sqlx::query_as::<_, Clip>(
        "INSERT INTO clips (id, project_id, track_id, asset_id, name, track_position_ms, in_point_ms, out_point_ms, duration_ms) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"
    )
    .bind(clip_id)
    .bind(project_id)
    .bind(track_id)
    .bind(asset_id)
    .bind(payload["name"].as_str().unwrap_or("Untitled Clip"))
    .bind(payload["track_position_ms"].as_i64().unwrap_or(0) as i32)
    .bind(payload["in_point_ms"].as_i64().unwrap_or(0) as i32)
    .bind(payload["out_point_ms"].as_i64().unwrap_or(1000) as i32)
    .bind(payload["duration_ms"].as_i64().unwrap_or(1000) as i32)
    .fetch_one(&pool)
    .await?;

    Ok((axum::http::StatusCode::CREATED, Json(clip)))
}

/* 3. อัปเดต Clip (Move/Resize) */
#[derive(Deserialize)]
pub struct UpdateClipRequest {
    pub track_id: Option<Uuid>,
    pub track_position_ms: Option<i32>,
    pub in_point_ms: Option<i32>,
    pub out_point_ms: Option<i32>,
    pub duration_ms: Option<i32>,
}

pub async fn update_clip(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path((project_id, clip_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateClipRequest>,
) -> Result<Json<Clip>, AppError> {
    check_project_access(&pool, project_id, user_id).await?;

    let clip = sqlx::query_as::<_, Clip>(
        "UPDATE clips SET 
            track_id = COALESCE($1, track_id),
            track_position_ms = COALESCE($2, track_position_ms),
            in_point_ms = COALESCE($3, in_point_ms),
            out_point_ms = COALESCE($4, out_point_ms),
            duration_ms = COALESCE($5, duration_ms),
            updated_at = NOW()
         WHERE id = $6 AND project_id = $7 AND deleted_at IS NULL
         RETURNING *"
    )
    .bind(payload.track_id)
    .bind(payload.track_position_ms)
    .bind(payload.in_point_ms)
    .bind(payload.out_point_ms)
    .bind(payload.duration_ms)
    .bind(clip_id)
    .bind(project_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Clip not found".into()))?;

    Ok(Json(clip))
}

/* 5. ลบ Clip (Soft Delete) */
pub async fn delete_clip(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path((project_id, clip_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    check_project_access(&pool, project_id, user_id).await?;

    let result = sqlx::query(
        "UPDATE clips SET deleted_at = NOW() WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL"
    )
    .bind(clip_id)
    .bind(project_id)
    .execute(&pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Clip not found".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn get_export_status(
    State(pool): State<PgPool>,
    State(s3): State<S3Client>,
    Claims(user_id): Claims,
    Path((project_id, export_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    check_project_access(&pool, project_id, user_id).await?;

    let job = sqlx::query_as::<_, shared::models::ExportJob>(
        "SELECT * FROM export_jobs WHERE id = $1 AND project_id = $2"
    )
    .bind(export_id)
    .bind(project_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Export job not found".into()))?;

    let mut job_json = serde_json::to_value(&job).map_err(|e| AppError::Internal(e.into()))?;

    if job.status == "completed" {
        if let Some(key_or_url) = &job.output_url {
            let key = if key_or_url.starts_with("http") {
                /* Legacy entries: extract the object key from the URL */
                let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
                let prefix = format!("/{}/", bucket);
                if let Some(pos) = key_or_url.find(&prefix) {
                    let after = &key_or_url[pos + prefix.len()..];
                    after.split('?').next().unwrap_or(after).to_string()
                } else {
                    key_or_url.clone()
                }
            } else {
                key_or_url.clone()
            };

            let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
            let filename = key.split('/').last().unwrap_or("video.mp4");
            let content_disposition = format!("attachment; filename=\"{}\"", filename);

            if let Ok(presigned_req) = s3
                .get_object()
                .bucket(&bucket)
                .key(&key)
                .response_content_disposition(content_disposition)
                .presigned(PresigningConfig::expires_in(Duration::from_secs(3600)).unwrap())
                .await
            {
                if let Some(obj) = job_json.as_object_mut() {
                    obj.insert(
                        "output_url".to_string(),
                        serde_json::Value::String(presigned_req.uri().to_string()),
                    );
                }
            }
        }
    }

    Ok(Json(job_json))
}

/* 4. Split Clip (Atomic Transaction) */
pub async fn split_clip(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path((project_id, clip_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<serde_json::Value>, /* { "split_time_ms": 5000 } */
) -> Result<impl IntoResponse, AppError> {
    check_project_access(&pool, project_id, user_id).await?;

    let split_time_ms = payload["split_time_ms"].as_i64().ok_or(AppError::Validation("Invalid split_time_ms".into()))? as i32;

    let mut tx = pool.begin().await?;

    /* ดึง Clip ต้นฉบับ */
    let original = sqlx::query_as::<_, Clip>(
        "SELECT * FROM clips WHERE id = $1 AND project_id = $2 AND deleted_at IS NULL FOR UPDATE"
    )
    .bind(clip_id)
    .bind(project_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Clip not found".into()))?;

    /* คำนวณจุดตัด */
    let relative_split_point = split_time_ms - original.track_position_ms;
    if relative_split_point <= 0 || relative_split_point >= original.duration_ms {
        return Err(AppError::Validation("Split point out of bounds".into()));
    }

    /* 1. หั่น Clip เดิม (ส่วนแรก) */
    let updated_original = sqlx::query_as::<_, Clip>(
        "UPDATE clips SET 
            out_point_ms = in_point_ms + $1,
            duration_ms = $1,
            updated_at = NOW()
         WHERE id = $2 RETURNING *"
    )
    .bind(relative_split_point)
    .bind(original.id)
    .fetch_one(&mut *tx)
    .await?;

    /* 2. สร้าง Clip ใหม่ (ส่วนที่สอง) */
    let new_clip_id = Uuid::new_v4();
    let new_clip = sqlx::query_as::<_, Clip>(
        "INSERT INTO clips (id, project_id, track_id, asset_id, name, track_position_ms, in_point_ms, out_point_ms, duration_ms) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *"
    )
    .bind(new_clip_id)
    .bind(project_id)
    .bind(original.track_id)
    .bind(original.asset_id)
    .bind(format!("{} (Part 2)", original.name))
    .bind(original.track_position_ms + relative_split_point)
    .bind(original.in_point_ms + relative_split_point)
    .bind(original.out_point_ms)
    .bind(original.duration_ms - relative_split_point)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(json!({
        "part1": updated_original,
        "part2": new_clip
    })))
}

/* Helper: ตรวจสอบสิทธิ์การเข้าถึง Project */
pub async fn check_project_access(pool: &PgPool, project_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let has_access = sqlx::query(
        "SELECT 1 FROM projects p 
         JOIN workspace_members wm ON p.workspace_id = wm.workspace_id
         WHERE p.id = $1 AND wm.user_id = $2 AND p.deleted_at IS NULL"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if has_access.is_none() {
        return Err(AppError::Forbidden("No access to this project".into()));
    }
    Ok(())
}

/* --- เดิม --- */

/* 1. ดึงรายการโปรเจกต์ทั้งหมดที่ User มีสิทธิ์เข้าถึง (อิงจาก Workspace) */
pub async fn list_projects(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
) -> Result<Json<Vec<Project>>, AppError> {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT p.* FROM projects p 
         JOIN workspaces w ON p.workspace_id = w.id
         JOIN workspace_members wm ON w.id = wm.workspace_id
         WHERE wm.user_id = $1 AND p.deleted_at IS NULL
         ORDER BY p.updated_at DESC"
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await?;

    Ok(Json(projects))
}

/* 2. สร้างโปรเจกต์ใหม่ พร้อมสร้าง 4 แทร็กเริ่มต้นอัตโนมัติ (Transaction) */
pub async fn create_project(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Json(payload): Json<CreateProjectRequest>,
) -> Result<impl IntoResponse, AppError> {
    /* เริ่มต้น Transaction */
    let mut tx = pool.begin().await?;

    /* ตรวจสอบสิทธิ์ใน Workspace ก่อน */
    let is_member = sqlx::query("SELECT 1 FROM workspace_members WHERE workspace_id = $1 AND user_id = $2")
        .bind(payload.workspace_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?;

    if is_member.is_none() {
        return Err(AppError::Forbidden("No access to this workspace".to_string()));
    }

    /* สร้าง Project */
    let project_id = Uuid::new_v4();
    let project = sqlx::query_as::<_, Project>(
        "INSERT INTO projects (id, workspace_id, name, description, created_by) 
         VALUES ($1, $2, $3, $4, $5) RETURNING *"
    )
    .bind(project_id)
    .bind(payload.workspace_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    /* สร้าง 2 Default Tracks (Video 1, Audio 1) */
    let default_tracks = vec![
        ("video", "Video 1", 1),
        ("audio", "Audio 1", 2),
    ];

    for (t_type, label, idx) in default_tracks {
        sqlx::query(
            "INSERT INTO tracks (id, project_id, type, label, order_index) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(Uuid::new_v4())
        .bind(project.id)
        .bind(t_type)
        .bind(label)
        .bind(idx)
        .execute(&mut *tx)
        .await?;
    }

    /* Commit Transaction */
    tx.commit().await?;

    Ok((axum::http::StatusCode::CREATED, Json(project)))
}

/* 4. อัปเดตข้อมูลโปรเจกต์ (เช่น เปลี่ยนชื่อ) */
pub async fn update_project(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<UpdateProjectRequest>,
) -> Result<Json<Project>, AppError> {
    check_project_access(&pool, project_id, user_id).await?;

    let project = sqlx::query_as::<_, Project>(
        "UPDATE projects SET 
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            updated_at = NOW()
         WHERE id = $3 AND deleted_at IS NULL
         RETURNING *"
    )
    .bind(payload.name)
    .bind(payload.description)
    .bind(project_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Project not found".into()))?;

    Ok(Json(project))
}

pub async fn get_workspace_plan(pool: &PgPool, project_id: Uuid) -> Result<String, AppError> {
    let plan: String = sqlx::query_scalar(
        "SELECT w.plan FROM projects p JOIN workspaces w ON p.workspace_id = w.id WHERE p.id = $1"
    )
    .bind(project_id)
    .fetch_one(pool)
    .await
    .map_err(|_| AppError::NotFound("Workspace not found".into()))?;
    Ok(plan)
}

pub async fn check_rate_limit(
    redis: &redis::Client,
    workspace_id: Uuid,
    key_prefix: &str,
    limit: u32,
    window_secs: u64,
) -> Result<(), AppError> {
    let mut conn = redis.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis connection error: {}", e))))?;

    let key = format!("rate_limit:{}:{}", key_prefix, workspace_id);
    
    let count: u32 = conn.get(&key).await.unwrap_or(0);
    if count >= limit {
        return Err(AppError::Forbidden(format!("Rate limit exceeded for {}", key_prefix)));
    }

    let _: () = conn.incr(&key, 1).await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis incr error: {}", e))))?;
    
    /* Set TTL if it's a new key */
    if count == 0 {
        let _: () = conn.expire(&key, window_secs as i64).await
            .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis expire error: {}", e))))?;
    }

    Ok(())
}

#[derive(serde::Serialize, Clone)]
pub struct ClipWithWaveform {
    #[serde(flatten)]
    pub clip: Clip,
    pub waveform: Option<serde_json::Value>,
}

#[derive(serde::Serialize)]
pub struct TrackWithClipsAndWaveform {
    #[serde(flatten)]
    pub track: Track,
    pub clips: Vec<ClipWithWaveform>,
}

#[derive(serde::Serialize)]
pub struct TimelineResponseExtended {
    pub project: Project,
    pub tracks: Vec<TrackWithClipsAndWaveform>,
}

/* 3. ดึงข้อมูล Timeline แบบครบจบในชุดเดียว (Project + Tracks + Clips) */
pub async fn get_timeline(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
) -> Result<Json<TimelineResponseExtended>, AppError> {
    /* 1. ดึง Project Metadata (พร้อมเช็คสิทธิ์) */
    let project = sqlx::query_as::<_, Project>(
        "SELECT p.* FROM projects p 
         JOIN workspace_members wm ON p.workspace_id = wm.workspace_id
         WHERE p.id = $1 AND wm.user_id = $2 AND p.deleted_at IS NULL"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Project not found or access denied".to_string()))?;

    /* 2. ดึง Tracks ทั้งหมด */
    let tracks = sqlx::query_as::<_, Track>(
        "SELECT * FROM tracks WHERE project_id = $1 ORDER BY order_index ASC"
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await?;

    /* 3. ดึง Clips ทั้งหมด */
    let clips = sqlx::query_as::<_, Clip>(
        "SELECT * FROM clips WHERE project_id = $1 AND deleted_at IS NULL"
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await?;

    /* 4. ดึง Waveform data ทั้งหมดที่เกี่ยวข้องกับ Assets ในโปรเจกต์นี้ */
    let asset_ids: Vec<Uuid> = clips.iter().map(|c| c.asset_id).collect();
    let waveforms = if !asset_ids.is_empty() {
        sqlx::query_as::<_, AssetVariant>(
            "SELECT * FROM asset_variants WHERE asset_id = ANY($1) AND type = 'waveform_data'"
        )
        .bind(&asset_ids)
        .fetch_all(&pool)
        .await?
    } else {
        Vec::new()
    };

    /* 5. ประกอบร่าง Unified Structure */
    let mut track_with_clips = Vec::new();
    for track in tracks {
        let mut track_clips = Vec::new();
        for clip in clips.iter().filter(|c| c.track_id == track.id) {
            let waveform = waveforms.iter()
                .find(|w| w.asset_id == clip.asset_id)
                .map(|w| w.metadata.clone());

            track_clips.push(ClipWithWaveform {
                clip: clip.clone(),
                waveform,
            });
        }

        track_with_clips.push(TrackWithClipsAndWaveform {
            track: track.clone(),
            clips: track_clips,
        });
    }

    Ok(Json(TimelineResponseExtended {
        project,
        tracks: track_with_clips,
    }))
}

/* 5. สร้าง Export Job */
#[derive(serde::Deserialize)]
pub struct ExportRequest {
    pub format: String,     /* เช่น "mp4" */
    pub resolution: String, /* เช่น "1080p" */
}

pub async fn create_export(
    State(pool): State<PgPool>,
    State(redis_client): State<redis::Client>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
    Json(payload): Json<ExportRequest>,
) -> Result<impl IntoResponse, AppError> {
    /* 1. ตรวจสอบสิทธิ์ Project */
    check_project_access(&pool, project_id, user_id).await?;

    /* Rate Limiting for Concurrent Exports */
    let plan = get_workspace_plan(&pool, project_id).await?;
    let (workspace_id,): (Uuid,) = sqlx::query_as("SELECT workspace_id FROM projects WHERE id = $1")
        .bind(project_id).fetch_one(&pool).await
        .map_err(|_| AppError::NotFound("Workspace ID not found".into()))?;

    let concurrent_limit = match plan.as_str() {
        "free" => 2,
        "pro" => 10,
        "team" => 30,
        _ => 2,
    };

    /* Check concurrent exports via DB for reliability */
    let active_exports: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM export_jobs ej 
         JOIN projects p ON ej.project_id = p.id 
         WHERE p.workspace_id = $1 AND ej.status IN ('queued', 'processing')"
    )
    .bind(workspace_id)
    .fetch_one(&pool)
    .await
    .map_err(|e| AppError::Internal(e.into()))?;

    if active_exports.0 >= concurrent_limit as i64 {
        return Err(AppError::Forbidden(format!("Concurrent export limit reached for plan {}", plan)));
    }

    /* 2. สร้าง Export Job record */
    let export_id = Uuid::now_v7();
    let idempotency_key = format!("export-{}", export_id);

    let mut tx = pool.begin().await?;

    sqlx::query(
        "INSERT INTO export_jobs (id, project_id, requested_by, format, resolution, quality, status, progress_percent, idempotency_key) 
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(export_id)
    .bind(project_id)
    .bind(user_id)
    .bind(&payload.format)
    .bind(&payload.resolution)
    .bind("high")
    .bind("queued")
    .bind(0)
    .bind(&idempotency_key)
    .execute(&mut *tx)
    .await?;

    /* 3. ส่งงานเข้า Redis */
    let job_payload = JobPayload::RenderExport {
        project_id,
        export_id,
        idempotency_key: idempotency_key.clone(),
        attempts: 0,
    };

    let mut conn = redis_client.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis connection error: {}", e))))?;

    let _: () = conn.lpush("queue:video_pipeline", serde_json::to_string(&job_payload).unwrap()).await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis push error: {}", e))))?;

    tx.commit().await?;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(json!({
            "exportId": export_id,
            "status": "queued"
        })),
    ))
}

pub async fn cancel_export(
    State(pool): State<PgPool>,
    State(redis_client): State<redis::Client>,
    Claims(user_id): Claims,
    Path((project_id, export_id)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, AppError> {
    /* 1. ตรวจสอบสิทธิ์ Project */
    check_project_access(&pool, project_id, user_id).await?;

    /* 2. อัปเดตสถานะเป็น cancelled */
    let result = sqlx::query("UPDATE export_jobs SET status = 'cancelled', updated_at = NOW() WHERE id = $1 AND project_id = $2 AND status IN ('queued', 'processing')")
        .bind(export_id)
        .bind(project_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Export job not found or cannot be cancelled".into()));
    }

    /* 3. ส่งสัญญาณ Cancel ไปที่ Redis Pub/Sub */
    let mut conn = redis_client.get_multiplexed_async_connection().await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis connection error: {}", e))))?;

    let _: () = redis::cmd("PUBLISH")
        .arg("channel:export:cancel")
        .arg(export_id.to_string())
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Internal(anyhow::Error::msg(format!("Redis publish error: {}", e))))?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/* 6. ลบ Project (Hard Delete) */
pub async fn delete_project_handler(
    State(pool): State<PgPool>,
    Extension(s3): Extension<aws_sdk_s3::Client>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    /* ตรวจสอบสิทธิ์ */
    check_project_access(&pool, project_id, user_id).await?;

    /* 1. ดึงข้อมูล Assets และ Exports ที่เกี่ยวข้องเพื่อไปลบใน S3 */
    let object_keys: Vec<String> = sqlx::query_scalar(
        "SELECT original_url FROM assets WHERE project_id = $1"
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await?;

    let export_keys: Vec<String> = sqlx::query_scalar(
        "SELECT output_url FROM export_jobs WHERE project_id = $1 AND output_url IS NOT NULL"
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await?;

    /* 2. ลบไฟล์ใน S3/MinIO */
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    
    /* ลบ Assets */
    for key in object_keys {
        let _ = s3.delete_object().bucket(&bucket).key(&key).send().await;
    }

    /* ลบ Exports */
    for url in export_keys {
        /* แยก Key ออกจาก URL (ตัวอย่าง: http://.../bucket/exports/...) */
        if let Some(key_start) = url.find("exports/") {
            let key = &url[key_start..];
            let _ = s3.delete_object().bucket(&bucket).key(key).send().await;
        }
    }

    /* 3. ลบข้อมูลใน Database (ON DELETE CASCADE จะจัดการ Tracks, Clips, Exports, Assets เอง) */
    let result = sqlx::query("DELETE FROM projects WHERE id = $1")
        .bind(project_id)
        .execute(&pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Project not found".into()));
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}
