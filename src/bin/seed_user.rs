//! Seed (or update) a user account. No public signup exists — accounts are
//! created with this tool.
//!
//! Usage:
//!   cargo run --bin seed_user -- <email> <password> [name]
//!
//! Re-running with an existing email updates that user's password/name (upsert),
//! so it doubles as a password reset for dev.

use prompt_eval::auth::hash_password;
use prompt_eval::db::create_pool;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: cargo run --bin seed_user -- <email> <password> [name]");
        std::process::exit(1);
    }
    let email = args[1].trim().to_lowercase();
    let password = &args[2];
    let name: Option<String> = args.get(3).cloned();

    let pool = create_pool().await.expect("failed to connect to database");

    let id = format!("u_{}", chrono::Utc::now().timestamp_micros());
    let hash = hash_password(password).expect("failed to hash password");

    let result = sqlx::query(
        r#"
        INSERT INTO users (id, email, password_hash, name, created_at)
        VALUES ($1, $2, $3, $4, NOW())
        ON CONFLICT (email)
        DO UPDATE SET password_hash = EXCLUDED.password_hash, name = EXCLUDED.name
        "#,
    )
    .bind(&id)
    .bind(&email)
    .bind(&hash)
    .bind(&name)
    .execute(&pool)
    .await;

    match result {
        Ok(_) => println!("✅ Seeded user: {email}"),
        Err(e) => {
            eprintln!("❌ Failed to seed user: {e}");
            std::process::exit(1);
        }
    }
}
