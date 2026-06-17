use crate::auth::{issue_token, verify_password};
use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginUser {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: LoginUser,
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    email: String,
    name: Option<String>,
    password_hash: String,
}

/// POST /api/auth/login
///
/// Verify an email/password pair against the `users` table and, on success,
/// return a signed JWT plus the user record. Returns 401 for both "no such
/// user" and "wrong password" so the response doesn't reveal which emails exist.
pub async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    let email = payload.email.trim().to_lowercase();

    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, email, name, password_hash FROM users WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        eprintln!("login db error: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    if !verify_password(&payload.password, &user.password_hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = issue_token(&user.id, &user.email)?;

    Ok(Json(LoginResponse {
        token,
        user: LoginUser {
            id: user.id,
            email: user.email,
            name: user.name,
        },
    }))
}
