//! Authentication primitives: password hashing, JWT issue/verify, and the
//! `AuthUser` extractor that gates protected handlers.
//!
//! The browser talks to this API directly (separate origin from the Next.js
//! frontend), so identity rides in an `Authorization: Bearer <jwt>` header that
//! every protected handler verifies via the `AuthUser` extractor.

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::{header::AUTHORIZATION, request::Parts, StatusCode};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Hash a plaintext password into an argon2 PHC string for storage.
/// Each call uses a fresh random salt. Used by the `seed_user` binary.
#[allow(dead_code)]
pub fn hash_password(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
}

/// Verify a plaintext password against a stored argon2 PHC hash.
/// Returns false on any parse/verify failure — never panics.
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => false,
    }
}

// ── JWT ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject — the user id.
    pub sub: String,
    pub email: String,
    /// Expiry, unix seconds.
    pub exp: usize,
}

/// HS256 signing key. Falls back to a dev value so the app boots without config;
/// set JWT_SECRET in any real environment.
fn jwt_secret() -> String {
    std::env::var("JWT_SECRET").unwrap_or_else(|_| "dev-jwt-secret-change-me".to_string())
}

/// Issue a 7-day token for a user.
pub fn issue_token(user_id: &str, email: &str) -> Result<String, StatusCode> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(7)).timestamp() as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret().as_bytes()),
    )
    .map_err(|e| {
        eprintln!("jwt encode error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn verify_token(token: &str) -> Option<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret().as_bytes()),
        &Validation::default(),
    )
    .map(|data| data.claims)
    .ok()
}

// ── Extractor ─────────────────────────────────────────────────────────────────

/// The authenticated user, pulled from the `Authorization: Bearer <jwt>` header.
/// Any handler that takes this parameter requires a valid token — a missing or
/// invalid token yields `401 Unauthorized` before the handler body runs.
pub struct AuthUser {
    pub user_id: String,
    #[allow(dead_code)]
    pub email: String,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(StatusCode::UNAUTHORIZED)?
            .trim();
        let claims = verify_token(token).ok_or(StatusCode::UNAUTHORIZED)?;
        Ok(AuthUser {
            user_id: claims.sub,
            email: claims.email,
        })
    }
}
