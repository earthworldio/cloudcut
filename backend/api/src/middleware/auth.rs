use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use shared::models::JwtClaims;
use std::env;
use uuid::Uuid;
use crate::error::AppError;

/* โครงสร้างสำหรับเก็บ User ID ที่ได้จาก Token */
pub struct Claims(pub Uuid);

/* Implement FromRequestParts เพื่อให้ Axum ใช้เป็น Extractor ได้ */
#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        /* 1. ดึง Token จาก Header Authorization */
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        /* 2. ตรวจสอบและแกะข้อมูลจาก JWT */
        let jwt_secret = env::var("JWT_SECRET").unwrap_or_else(|_| "secret".to_string());
        let token_data = decode::<JwtClaims>(
            auth_header,
            &DecodingKey::from_secret(jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        /* 3. ส่ง User ID (sub) กลับไป */
        Ok(Claims(token_data.claims.sub))
    }
}
