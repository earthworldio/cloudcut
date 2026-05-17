use axum::{
    extract::{Path, State},
    Json,
    http::StatusCode,
    response::IntoResponse,
};
use shared::models::{Project, CreateProjectRequest, TimelineResponse, TrackWithClips, Track, Clip};
use crate::middleware::auth::Claims;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

/* 1. ดึงรายการโปรเจกต์ทั้งหมดที่ User มีสิทธิ์เข้าถึง (อิงจาก Workspace) */
pub async fn list_projects(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
) -> impl IntoResponse {
    let projects = sqlx::query_as::<_, Project>(
        "SELECT p.* FROM projects p 
         JOIN workspaces w ON p.workspace_id = w.id
         JOIN workspace_members wm ON w.id = wm.workspace_id
         WHERE wm.user_id = $1 AND p.deleted_at IS NULL
         ORDER BY p.updated_at DESC"
    )
    .bind(user_id)
    .fetch_all(&pool)
    .await;

    match projects {
        Ok(p) => (StatusCode::OK, Json(p)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response(),
    }
}

/* 2. สร้างโปรเจกต์ใหม่ พร้อมสร้าง 4 แทร็กเริ่มต้นอัตโนมัติ (Transaction) */
pub async fn create_project(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Json(payload): Json<CreateProjectRequest>,
) -> impl IntoResponse {
    /* เริ่มต้น Transaction */
    let mut tx = match pool.begin().await {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to start transaction").into_response(),
    };

    /* ตรวจสอบสิทธิ์ใน Workspace ก่อน */
    let is_member = sqlx::query("SELECT 1 FROM workspace_members WHERE workspace_id = $1 AND user_id = $2")
        .bind(payload.workspace_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await;

    if let Ok(None) = is_member {
        return (StatusCode::FORBIDDEN, "No access to this workspace").into_response();
    }

    /* สร้าง Project */
    let project_id = Uuid::new_v4();
    let project_result = sqlx::query_as::<_, Project>(
        "INSERT INTO projects (id, workspace_id, name, description, created_by) 
         VALUES ($1, $2, $3, $4, $5) RETURNING *"
    )
    .bind(project_id)
    .bind(payload.workspace_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await;

    let project = match project_result {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create project: {}", e)).into_response(),
    };

    /* สร้าง 4 Default Tracks (Video 2, Video 1, Audio 1, Audio 2) */
    let default_tracks = vec![
        ("video", "Video 2 (Overlay)", 1),
        ("video", "Video 1 (Main)", 2),
        ("audio", "Audio 1 (Voice)", 3),
        ("audio", "Audio 2 (Music)", 4),
    ];

    for (t_type, label, idx) in default_tracks {
        let res = sqlx::query(
            "INSERT INTO tracks (id, project_id, type, label, order_index) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(Uuid::new_v4())
        .bind(project.id)
        .bind(t_type)
        .bind(label)
        .bind(idx)
        .execute(&mut *tx)
        .await;

        if let Err(e) = res {
            return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create default tracks: {}", e)).into_response();
        }
    }

    /* Commit Transaction */
    if let Err(_) = tx.commit().await {
        return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to commit transaction").into_response();
    }

    (StatusCode::CREATED, Json(project)).into_response()
}

/* 3. ดึงข้อมูล Timeline แบบครบจบในชุดเดียว (Project + Tracks + Clips) */
pub async fn get_timeline(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
    Path(project_id): Path<Uuid>,
) -> impl IntoResponse {
    /* 1. ดึง Project Metadata (พร้อมเช็คสิทธิ์) */
    let project = sqlx::query_as::<_, Project>(
        "SELECT p.* FROM projects p 
         JOIN workspace_members wm ON p.workspace_id = wm.workspace_id
         WHERE p.id = $1 AND wm.user_id = $2 AND p.deleted_at IS NULL"
    )
    .bind(project_id)
    .bind(user_id)
    .fetch_optional(&pool)
    .await;

    let project = match project {
        Ok(Some(p)) => p,
        Ok(None) => return (StatusCode::NOT_FOUND, "Project not found or access denied").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", e)).into_response(),
    };

    /* 2. ดึง Tracks ทั้งหมด */
    let tracks = sqlx::query_as::<_, Track>(
        "SELECT * FROM tracks WHERE project_id = $1 ORDER BY order_index ASC"
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await;

    let tracks = match tracks {
        Ok(t) => t,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch tracks: {}", e)).into_response(),
    };

    /* 3. ดึง Clips ทั้งหมด */
    let clips = sqlx::query_as::<_, Clip>(
        "SELECT * FROM clips WHERE project_id = $1 AND deleted_at IS NULL"
    )
    .bind(project_id)
    .fetch_all(&pool)
    .await;

    let clips = match clips {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch clips: {}", e)).into_response(),
    };

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

    Json(TimelineResponse {
        project,
        tracks: track_with_clips,
    }).into_response()
}
