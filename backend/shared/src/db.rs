use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::env;
use dotenvy::dotenv;

/* ฟังก์ชันสำหรับเชื่อมต่อ Database โดยอ่านค่าจาก .env */
pub async fn establish_connection() -> Result<PgPool, sqlx::Error> {
    /* โหลดไฟล์ .env */
    dotenv().ok();

    /* อ่าน DATABASE_URL จาก environment variable */
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");

    /* สร้าง Pool สำหรับเชื่อมต่อกับ PostgreSQL */
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    Ok(pool)
}
