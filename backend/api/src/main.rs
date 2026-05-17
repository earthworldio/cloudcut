mod handlers;
mod middleware;

use axum::{
    routing::{get, post},
    Router,
};
use handlers::auth::{register, login};
use handlers::projects::{list_projects, create_project, get_timeline};
use shared::establish_connection;
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use axum::http::{HeaderValue, Method};

#[tokio::main]
async fn main() {
    /* 1. เชื่อมต่อฐานข้อมูล */
    let pool = establish_connection()
        .await
        .expect("Failed to connect to database");

    /* 2. ตั้งค่า CORS (อนุญาตให้ Frontend เข้าถึงได้) */
    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_headers(Any);

    /* 3. ตั้งค่า Router */
    let app = Router::new()
        /* Auth Routes */
        .route("/api/auth/register", post(register))
        .route("/api/auth/login", post(login))
        
        /* Project & Timeline Routes */
        .route("/api/projects", get(list_projects).post(create_project))
        .route("/api/projects/:id/timeline", get(get_timeline))
        
        .layer(cors)
        .with_state(pool);

    /* 4. เริ่ม Server */
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Server started at http://{}", addr);
    println!("📡 API Endpoints:");
    println!("   - POST /api/auth/register");
    println!("   - POST /api/auth/login");
    println!("   - GET  /api/projects (Protected)");
    println!("   - POST /api/projects (Protected)");
    println!("   - GET  /api/projects/:id/timeline (Protected)");
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
