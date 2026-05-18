mod handlers;
mod middleware;
mod error;

use axum::{
    routing::{get, post, patch, delete},
    Router,
    extract::FromRef,
};
use handlers::auth::{register, login, me};
use handlers::projects::{
    list_projects, create_project, get_timeline, update_project, update_clip, delete_clip, 
    split_clip, create_export, cancel_export, get_export_status, create_track, create_clip,
    delete_project_handler, delete_workspace
};
use handlers::assets::{get_presigned_url, confirm_upload, list_assets};
use shared::establish_connection;
use std::net::SocketAddr;
use std::time::Duration;
use tower_http::cors::{CorsLayer, Any};
use axum::http::{HeaderValue, Method};
use sqlx::PgPool;
use uuid::Uuid;

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
        db: pool.clone(),
        s3: s3_client.clone(),
        redis: redis_client,
    };

    /* 3. ตั้งค่า CORS (อนุญาตให้ Frontend เข้าถึงได้) */
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
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
        .route("/projects/:id", get(get_timeline).patch(update_project).delete(delete_project_handler))
        .route("/projects/:id/timeline", get(get_timeline))
        .route("/projects/:id/tracks", post(create_track))
        .route("/projects/:id/clips", post(create_clip))
        .route("/projects/:id/clips/:clip_id", patch(update_clip).delete(delete_clip))
        .route("/projects/:id/clips/:clip_id/split", post(split_clip))
        .route("/projects/:id/exports", post(create_export))
        .route("/projects/:id/exports/:export_id", get(get_export_status))
        .route("/projects/:id/exports/:export_id/cancel", post(cancel_export))
        
        /* Workspace Routes */
        .route("/workspaces/:id", delete(delete_workspace))
        
        /* Asset Routes */
        .route("/assets/presigned-url", post(get_presigned_url))
        .route("/assets/confirm-upload", post(confirm_upload))
        .route("/assets", get(list_assets));

    let app = Router::new()
        .nest("/api", api_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    /* 5. เริ่ม Server */
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Server started at http://{}", addr);

    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    /* 6. Start Scheduled Cleanup Background Loop */
    let cleanup_pool = pool.clone();
    let cleanup_s3 = s3_client.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600)); /* Every hour */
        loop {
            interval.tick().await;
            info!("Running scheduled cleanup job...");
            if let Err(e) = run_cleanup(&cleanup_pool, &cleanup_s3).await {
                error!("Scheduled cleanup failed: {}", e);
            }
        }
    });

    axum::serve(listener, app).await.unwrap();
}

async fn run_cleanup(pool: &sqlx::PgPool, s3: &aws_sdk_s3::Client) -> anyhow::Result<()> {
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "cloudcut-assets".to_string());
    
    /* 1. ลบ soft-deleted projects ที่เกิน 30 วัน */
    let deleted_projects = sqlx::query("DELETE FROM projects WHERE deleted_at < NOW() - INTERVAL '30 days'")
        .execute(pool).await?.rows_affected();

    /* 2. ลบ export files ที่หมดอายุ */
    let expired_exports = sqlx::query_as::<_, (Uuid, String)>("SELECT id, output_url FROM export_jobs WHERE expires_at < NOW() AND status = 'completed'")
        .fetch_all(pool).await?;
    
    let mut deleted_exports_count = 0;
    for (id, url) in &expired_exports {
        /* Extract key from URL or store key in DB */
        /* Assuming key is exports/{project_id}/{export_id}.mp4 */
        /* For simplicity, we'll try to delete if we can find the key */
        if let Some(key_start) = url.find("exports/") {
            let key = &url[key_start..];
            if let Some(key_end) = key.find('?') {
                let clean_key = &key[..key_end];
                if let Err(e) = s3.delete_object().bucket(&bucket).key(clean_key).send().await {
                    warn!("Failed to delete S3 object {}: {}", clean_key, e);
                } else {
                    deleted_exports_count += 1;
                }
            }
        }
        sqlx::query("DELETE FROM export_jobs WHERE id = $1").bind(id).execute(pool).await?;
    }

    /* 3. ลบ orphaned assets (ไม่ได้ถูกใช้ในคลิปใดๆ และเก่ากว่า 7 วัน) */
    let deleted_assets = sqlx::query(
        "DELETE FROM assets a 
         WHERE NOT EXISTS (SELECT 1 FROM clips c WHERE c.asset_id = a.id) 
         AND a.created_at < NOW() - INTERVAL '7 days'"
    )
    .execute(pool).await?.rows_affected();

    info!(
        deleted_projects = deleted_projects,
        deleted_exports = deleted_exports_count,
        deleted_assets = deleted_assets,
        "Cleanup summary"
    );

    Ok(())
}

use tracing::{info, error, warn};
