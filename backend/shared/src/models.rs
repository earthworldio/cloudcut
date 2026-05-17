use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;
use validator::Validate;

/* Model สำหรับตาราง users */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub oauth_provider: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/* Model สำหรับตาราง workspaces */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub plan: String,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/* Model สำหรับตาราง workspace_members */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WorkspaceMember {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* Model สำหรับตาราง invitations */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Invitation {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/* Model สำหรับตาราง projects */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Project {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub settings: Value,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/* Model สำหรับตาราง assets */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Asset {
    pub id: Uuid,
    pub project_id: Uuid,
    pub uploaded_by: Uuid,
    pub r#type: String, /* ใช้ r# เพราะ type เป็น keyword ใน Rust */
    pub original_url: String,
    pub status: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/* Model สำหรับตาราง asset_variants */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AssetVariant {
    pub id: Uuid,
    pub asset_id: Uuid,
    pub r#type: String,
    pub url: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

/* Model สำหรับตาราง tracks */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Track {
    pub id: Uuid,
    pub project_id: Uuid,
    pub r#type: String,
    pub label: String,
    pub order_index: i32,
    pub is_locked: bool,
    pub is_muted: bool,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* Model สำหรับตาราง clips */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Clip {
    pub id: Uuid,
    pub project_id: Uuid,
    pub track_id: Uuid,
    pub asset_id: Uuid,
    pub name: String,
    pub track_position_ms: i32,
    pub in_point_ms: i32,
    pub out_point_ms: i32,
    pub duration_ms: i32,
    pub transform: Value,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

/* Model สำหรับตาราง clip_effects */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ClipEffect {
    pub id: Uuid,
    pub clip_id: Uuid,
    pub r#type: String,
    pub order_index: i32,
    pub params: Value,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* Model สำหรับตาราง transitions */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Transition {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_clip_id: Uuid,
    pub to_clip_id: Uuid,
    pub r#type: String,
    pub duration_ms: i32,
    pub params: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* Model สำหรับตาราง text_overlays */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TextOverlay {
    pub id: Uuid,
    pub project_id: Uuid,
    pub track_position_ms: i32,
    pub duration_ms: i32,
    pub content: String,
    pub font_family: String,
    pub font_size: i32,
    pub font_color: String,
    pub position: Value,
    pub alignment: String,
    pub background_color: Option<String>,
    pub background_opacity: Option<f32>,
    pub animation: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* Model สำหรับตาราง export_jobs */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExportJob {
    pub id: Uuid,
    pub project_id: Uuid,
    pub requested_by: Uuid,
    pub format: String,
    pub resolution: String,
    pub quality: String,
    pub status: String,
    pub progress_percent: i32,
    pub output_url: Option<String>,
    pub output_file_size: Option<i64>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/* --- DTO สำหรับระบบ Authentication --- */

/* สำหรับรับข้อมูลตอน Register */
#[derive(Debug, Clone, serde::Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 8, message = "Password must be at least 8 characters long"))]
    pub password_plain: String,
    #[validate(length(min = 2, message = "Name must be at least 2 characters long"))]
    pub name: String,
}

/* สำหรับรับข้อมูลตอน Login */
#[derive(Debug, Clone, serde::Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    pub password_plain: String,
}

/* สำหรับตอบกลับเมื่อ Auth สำเร็จ */
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

/* ข้อมูล User ที่จะส่งกลับไป (ไม่ส่ง password hash) */
#[derive(Debug, Clone, serde::Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub workspace_id: Option<Uuid>, /* เพิ่มฟิลด์นี้ */
}

/* โครงสร้าง Claims สำหรับฝังใน JWT Token */
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct JwtClaims {
    pub sub: Uuid, /* User ID */
    pub exp: i64,  /* วันหมดอายุ */
    pub iat: i64,  /* วันที่ออกตั๋ว */
}

/* --- DTO สำหรับระบบ Project & Timeline --- */

/* สำหรับรับข้อมูลการสร้าง Project */
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CreateProjectRequest {
    pub workspace_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

/* --- DTO สำหรับระบบ Assets --- */

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresignedUrlRequest {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub project_id: Uuid,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresignedUrlResponse {
    pub upload_url: String,
    pub asset_id: Uuid,
    pub object_key: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmUploadRequest {
    pub asset_id: Uuid,
    pub object_key: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub project_id: Uuid,
}

/* สำหรับรับข้อมูลการอัปเดต Project */
#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/* --- Job Payload Enum (Spec 3.3) --- */

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum JobPayload {
    #[serde(rename = "extract_metadata")]
    ExtractMetadata {
        asset_id: Uuid,
        input_url: String,
        idempotency_key: String,
        #[serde(default)]
        attempts: u32,
    },
    #[serde(rename = "generate_proxy")]
    GenerateProxy {
        asset_id: Uuid,
        input_url: String,
        idempotency_key: String,
        #[serde(default)]
        attempts: u32,
    },
    #[serde(rename = "generate_thumbnails")]
    GenerateThumbnails {
        asset_id: Uuid,
        input_url: String,
        idempotency_key: String,
        #[serde(default)]
        attempts: u32,
    },
    #[serde(rename = "extract_waveform")]
    ExtractWaveform {
        asset_id: Uuid,
        input_url: String,
        idempotency_key: String,
        #[serde(default)]
        attempts: u32,
    },
    #[serde(rename = "render_export")]
    RenderExport {
        project_id: Uuid,
        export_id: Uuid,
        idempotency_key: String,
        #[serde(default)]
        attempts: u32,
    },
}

/* สำหรับส่งข้อมูล Timeline แบบรวมศูนย์ (Unified Structure) */
#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineResponse {
    pub project: Project,
    pub tracks: Vec<TrackWithClips>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TrackWithClips {
    #[serde(flatten)]
    pub track: Track,
    pub clips: Vec<Clip>,
}

/* Model สำหรับตาราง processing_jobs */
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProcessingJob {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub payload: Value,
    pub attempts: i32,
    pub max_attempts: i32,
    pub next_run_at: DateTime<Utc>,
    pub error_message: Option<String>,
    pub idempotency_key: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
