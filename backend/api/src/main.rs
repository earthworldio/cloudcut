mod handlers;
mod middleware;

use axum::{
    routing::{get, post},
    Router,
    Json,
};
use handlers::auth::{register, login};
use middleware::auth::Claims;
use shared::establish_connection;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    /* 1. เชื่อมต่อฐานข้อมูล */
    let pool = establish_connection()
        .await
        .expect("Failed to connect to database");

    /* 2. ตั้งค่า Router */
    let app = Router::new()
        /* Public routes */
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        /* Protected routes (ตัวอย่าง) */
        .route("/api/protected", get(protected_handler))
        .layer(CorsLayer::permissive())
        .with_state(pool);

    /* 3. เริ่ม Server */
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Server started at http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/* Handler ตัวอย่างสำหรับทดสอบ Middleware */
async fn protected_handler(Claims(user_id): Claims) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "success",
        "message": "You have access!",
        "user_id": user_id
    }))
}
