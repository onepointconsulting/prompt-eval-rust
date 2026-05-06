mod db;
mod handlers;
mod models;
mod routes; // NEW
mod utils;

use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    // Create database connection pool
    let pool = db::create_pool()
        .await
        .expect("Failed to create database pool");

    let cors = CorsLayer::permissive();
    let app = routes::create_router(pool).layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();

    println!("🚀 Rust API server running on http://127.0.0.1:3001");

    axum::serve(listener, app).await.unwrap();
}
