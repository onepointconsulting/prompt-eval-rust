mod app_state;
mod bootstrap;
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

    bootstrap::maybe_run(&pool)
        .await
        .expect("Bootstrap script failed");

    let llm = llm::anthropic_client::AnthropicClient::from_env()
        .expect("ANTHROPIC_API_KEY, ANTHROPIC_MODEL_HAIKU, and ANTHROPIC_MODEL_SONNET must be set");

    let context = llm::context_client::ContextClient::from_env();
    if context.is_some() {
        println!("🔌 Context engine connected (CONTEXT_ENGINE_URL set)");
    } else {
        println!("ℹ️  Context engine not configured — evaluations will run without KB context");
    }

    let state = AppState::new(pool, llm, context);

    let cors = CorsLayer::permissive();
    let app = routes::create_router(state).layer(cors);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".into());
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    println!("🚀 Rust API server running on http://{addr}");

    axum::serve(listener, app).await.unwrap();
}
