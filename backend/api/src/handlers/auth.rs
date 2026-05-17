use axum::{extract::State, Json, response::IntoResponse};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, Header, EncodingKey};
use shared::models::{RegisterRequest, LoginRequest, AuthResponse, UserResponse, JwtClaims, User};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};
use std::env;
use validator::Validate;
use crate::error::AppError;
use crate::middleware::auth::Claims;

/* Handler สำหรับดูข้อมูลตัวเอง (GET /auth/me) */
pub async fn me(
    State(pool): State<PgPool>,
    Claims(user_id): Claims,
) -> Result<Json<UserResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1 AND deleted_at IS NULL")
        .bind(user_id)
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

    /* หา workspace ตัวแรกที่ user สังกัด */
    let workspace_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT workspace_id FROM workspace_members WHERE user_id = $1 LIMIT 1"
    )
    .bind(user_id)
    .fetch_optional(&pool)
    .await?;

    Ok(Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
        workspace_id,
    }))
}

/* Handler สำหรับการสมัครสมาชิก */
pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    /* 1. Validate ข้อมูล */
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    /* 2. แฮชรหัสผ่าน */
    let hashed_password = hash(payload.password_plain, DEFAULT_COST)
        .map_err(|e| anyhow::anyhow!("Error hashing password: {}", e))?;

    /* เริ่มต้น Transaction */
    let mut tx = pool.begin().await?;

    /* 3. บันทึก User ลงฐานข้อมูล */
    let user_id = Uuid::new_v4();
    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(user_id)
    .bind(&payload.email)
    .bind(&hashed_password)
    .bind(&payload.name)
    .fetch_one(&mut *tx)
    .await?;

    /* 4. สร้าง Workspace เริ่มต้นให้ทันที พร้อม Role 'owner' */
    let workspace_id = Uuid::new_v4();
    let slug = format!("{}-workspace-{}", payload.name.to_lowercase().replace(" ", "-"), Uuid::new_v4().to_string()[..8].to_string());
    
    sqlx::query(
        "INSERT INTO workspaces (id, name, slug, owner_id) VALUES ($1, $2, $3, $4)"
    )
    .bind(workspace_id)
    .bind(format!("{}'s Workspace", payload.name))
    .bind(slug)
    .bind(user.id)
    .execute(&mut *tx)
    .await?;

    /* ผูก User เข้ากับ Workspace ในฐานะ 'owner' */
    sqlx::query(
        "INSERT INTO workspace_members (id, workspace_id, user_id, role) VALUES ($1, $2, $3, $4)"
    )
    .bind(Uuid::new_v4())
    .bind(workspace_id)
    .bind(user.id)
    .bind("owner")
    .execute(&mut *tx)
    .await?;

    /* Commit Transaction */
    tx.commit().await?;

    Ok((axum::http::StatusCode::CREATED, Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
        workspace_id: Some(workspace_id),
    })))
}

/* Handler สำหรับการเข้าสู่ระบบ */
pub async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    /* 1. Validate ข้อมูล */
    payload.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    /* 2. ค้นหา User จาก Email */
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1 AND deleted_at IS NULL")
        .bind(&payload.email)
        .fetch_optional(&pool)
        .await?
        .ok_or(AppError::Unauthorized)?;

    /* 3. ตรวจสอบรหัสผ่าน */
    let is_valid = verify(payload.password_plain, &user.password_hash)
        .map_err(|e| anyhow::anyhow!("Error verifying password: {}", e))?;

    if !is_valid {
        return Err(AppError::Unauthorized);
    }

    /* 4. สร้าง JWT Token (มีอายุ 24 ชม.) */
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let expiration = Utc::now() + Duration::hours(24);
    let claims = JwtClaims {
        sub: user.id,
        exp: expiration.timestamp(),
        iat: Utc::now().timestamp(),
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_ref()))
        .map_err(|e| anyhow::anyhow!("Error generating token: {}", e))?;

    /* หา workspace_id ตัวแรก */
    let workspace_id: Option<Uuid> = sqlx::query_scalar(
        "SELECT workspace_id FROM workspace_members WHERE user_id = $1 LIMIT 1"
    )
    .bind(user.id)
    .fetch_optional(&pool)
    .await?;

    /* 5. ส่ง Token และข้อมูล User กลับไป */
    Ok(Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
            workspace_id,
        },
    }))
}
