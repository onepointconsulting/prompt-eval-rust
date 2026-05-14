mod app_state;
mod db;
mod handlers;
mod llm;
mod models;
mod routes;
mod utils;

use app_state::AppState;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let pool = db::create_pool()
        .await
        .expect("Failed to create database pool");

    let llm = llm::anthropic_client::AnthropicClient::from_env()
        .expect("ANTHROPIC_API_KEY, ANTHROPIC_MODEL_HAIKU, and ANTHROPIC_MODEL_SONNET must be set");

    let state = AppState::new(pool, llm);

    let cors = CorsLayer::permissive();
    let app = routes::create_router(state).layer(cors);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3001")
        .await
        .unwrap();

    println!("🚀 Rust API server running on http://127.0.0.1:3001");

    axum::serve(listener, app).await.unwrap();
}
