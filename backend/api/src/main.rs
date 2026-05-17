mod handlers;
mod middleware;
mod error;

use axum::{
    routing::{get, post, patch},
    Router,
    extract::FromRef,
};
use handlers::auth::{register, login, me};
use handlers::projects::{list_projects, create_project, get_timeline, update_project};
use shared::establish_connection;
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use axum::http::{HeaderValue, Method};
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub s3: aws_sdk_s3::Client,
    pub redis: redis::Client,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.db.clone()
    }
}

impl FromRef<AppState> for aws_sdk_s3::Client {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.s3.clone()
    }
}

impl FromRef<AppState> for redis::Client {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.redis.clone()
    }
}

use axum::http::Request;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    /* 1. เชื่อมต่อฐานข้อมูล */
    let pool = establish_connection()
        .await
        .expect("Failed to connect to database");

    /* 2. ตั้งค่า S3 Client */
    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string());
    let endpoint_url = std::env::var("AWS_ENDPOINT_URL").ok();
    let access_key = std::env::var("AWS_ACCESS_KEY_ID").ok();
    let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
    
    let mut s3_config_builder = aws_sdk_s3::config::Builder::from(&aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(region))
        .load()
        .await)
        .force_path_style(true); /* จำเป็นสำหรับ MinIO */
    
    if let Some(url) = endpoint_url {
        s3_config_builder = s3_config_builder.endpoint_url(url);
    }

    if let (Some(key), Some(secret)) = (access_key, secret_key) {
        s3_config_builder = s3_config_builder.credentials_provider(
            aws_sdk_s3::config::Credentials::new(key, secret, None, None, "static")
        );
    }
    
    let s3_client = aws_sdk_s3::Client::from_conf(s3_config_builder.build());

    /* 3. ตั้งค่า Redis Client */
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");

    let state = AppState {
        db: pool,
        s3: s3_client,
        redis: redis_client,
    };

    /* 3. ตั้งค่า CORS (อนุญาตให้ Frontend เข้าถึงได้) */
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5174".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    /* 4. ตั้งค่า Router */
    let api_routes = Router::new()
        /* Auth Routes */
        .route("/auth/register", post(register))
        .route("/auth/login", post(login))
        .route("/auth/me", get(me))
        
        /* Project & Timeline Routes */
        .route("/projects", get(list_projects).post(create_project))
        .route("/projects/:id", patch(update_project).get(get_timeline))
        .route("/projects/:id/timeline", get(get_timeline))
        .route("/projects/:id/tracks", post(handlers::projects::create_track))
        .route("/projects/:id/clips", post(handlers::projects::create_clip))
        .route("/projects/:id/clips/:clip_id", post(handlers::projects::update_clip))
        .route("/projects/:id/clips/:clip_id/split", post(handlers::projects::split_clip))
        .route("/projects/:id/exports", post(handlers::projects::create_export))
        
        /* Asset Routes */
        .route("/assets/presigned-url", post(handlers::assets::get_presigned_url))
        .route("/assets/confirm-upload", post(handlers::assets::confirm_upload))
        .route("/assets", get(handlers::assets::list_assets));

    let app = Router::new()
        .nest("/api", api_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    /* 5. เริ่ม Server */
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Server started at http://{}", addr);

    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
