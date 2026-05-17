use axum::{extract::State, Json, http::StatusCode, response::IntoResponse};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, Header, EncodingKey};
use shared::models::{RegisterRequest, LoginRequest, AuthResponse, UserResponse, JwtClaims, User};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};
use std::env;

/* Handler สำหรับการสมัครสมาชิก */
pub async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    /* 1. แฮชรหัสผ่าน */
    let hashed_password = match hash(payload.password_plain, DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Error hashing password").into_response(),
    };

    /* 2. บันทึก User ลงฐานข้อมูล */
    let user_id = Uuid::new_v4();
    let user_result = sqlx::query_as::<_, User>(
        "INSERT INTO users (id, email, password_hash, name) VALUES ($1, $2, $3, $4) RETURNING *"
    )
    .bind(user_id)
    .bind(&payload.email)
    .bind(&hashed_password)
    .bind(&payload.name)
    .fetch_one(&pool)
    .await;

    let user = match user_result {
        Ok(u) => u,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Error creating user: {}", e)).into_response(),
    };

    /* 3. สร้าง Workspace เริ่มต้นให้ทันที */
    let workspace_id = Uuid::new_v4();
    let slug = format!("{}-workspace", payload.name.to_lowercase().replace(" ", "-"));
    let ws_result = sqlx::query(
        "INSERT INTO workspaces (id, name, slug, owner_id) VALUES ($1, $2, $3, $4)"
    )
    .bind(workspace_id)
    .bind(format!("{}'s Workspace", payload.name))
    .bind(slug)
    .bind(user.id)
    .execute(&pool)
    .await;

    if let Err(e) = ws_result {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Error creating default workspace: {}", e)).into_response();
    }

    /* 4. ส่งข้อมูล User กลับไป (ไม่ส่งตั๋วในขั้นตอนนี้ หรือจะส่งก็ได้แล้วแต่ดีไซน์) */
    (StatusCode::CREATED, Json(UserResponse {
        id: user.id,
        email: user.email,
        name: user.name,
    })).into_response()
}

/* Handler สำหรับการเข้าสู่ระบบ */
pub async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    /* 1. ค้นหา User จาก Email */
    let user_result = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(&pool)
        .await;

    let user = match user_result {
        Ok(Some(u)) => u,
        Ok(None) => return (StatusCode::UNAUTHORIZED, "Invalid email or password").into_response(),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Database error").into_response(),
    };

    /* 2. ตรวจสอบรหัสผ่าน */
    let is_valid = match verify(payload.password_plain, &user.password_hash) {
        Ok(v) => v,
        Err(_) => false,
    };

    if !is_valid {
        return (StatusCode::UNAUTHORIZED, "Invalid email or password").into_response();
    }

    /* 3. สร้าง JWT Token (มีอายุ 24 ชม.) */
    let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
    let expiration = Utc::now() + Duration::hours(24);
    let claims = JwtClaims {
        sub: user.id,
        exp: expiration.timestamp(),
        iat: Utc::now().timestamp(),
    };

    let token = match encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_ref())) {
        Ok(t) => t,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Error generating token").into_response(),
    };

    /* 4. ส่ง Token และข้อมูล User กลับไป */
    Json(AuthResponse {
        token,
        user: UserResponse {
            id: user.id,
            email: user.email,
            name: user.name,
        },
    }).into_response()
}
