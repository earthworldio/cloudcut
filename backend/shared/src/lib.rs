pub mod models;
pub mod db;

/* Re-export เพื่อให้เรียกใช้งานได้ง่ายขึ้น */
pub use db::establish_connection;
