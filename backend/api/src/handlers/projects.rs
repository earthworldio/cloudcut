use axum::{
    extract::{Path, State},
    Json,
    response::IntoResponse,
};
use serde_json::json;
use shared::models::{Project, CreateProjectRequest, UpdateProjectRequest, TimelineResponse, TrackWithClips, Track, Clip};
use crate::middleware::auth::Claims;
use crate::error::AppError;
use sqlx::{PgPool};
use uuid::Uuid;
use serde::Deserialize;

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

    /* สร้าง 4 Default Tracks (Video 2, Video 1, Audio 1, Audio 2) */
    let default_tracks = vec![
        ("video", "Video 2 (Overlay)", 1),
        ("video", "Video 1 (Main)", 2),
        ("audio", "Audio 1 (Voice)", 3),
        ("audio", "Audio 2 (Music)", 4),
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

/* 3. ดึงข้อมูล Timeline แบบครบจบในชุดเดียว (Project + Tracks + Clips) */
pub async fn get_timeline(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
) -> Result<Json<TimelineResponse>, AppError> {
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

    /* 4. ประกอบร่าง Unified Structure */
    let mut track_with_clips = Vec::new();
    for track in tracks {
        let track_clips: Vec<Clip> = clips
            .iter()
            .filter(|c| c.track_id == track.id)
            .cloned()
            .collect();

        track_with_clips.push(TrackWithClips {
            track,
            clips: track_clips,
        });
    }

    Ok(Json(TimelineResponse {
        project,
        tracks: track_with_clips,
    }))
}
