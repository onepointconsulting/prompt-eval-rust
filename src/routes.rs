use crate::app_state::AppState;
use crate::handlers;
use axum::{
    routing::{get, post},
    Router,
};

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Evaluations
        .route("/api/evaluate", post(handlers::evaluations::run_evaluation))
        .route(
            "/api/evaluations",
            get(handlers::evaluations::list_evaluations),
        )
        .route(
            "/api/evaluations/:id",
            get(handlers::evaluations::get_evaluation),
        )
        // Prompts
        .route("/api/prompts", get(handlers::prompts::list))
        .route("/api/prompts", post(handlers::prompts::create))
        .route(
            "/api/prompts/generate",
            post(handlers::prompts::generate_prompt),
        )
        .route(
            "/api/prompts/:id",
            get(handlers::prompts::get)
                .put(handlers::prompts::update)
                .delete(handlers::prompts::delete),
        )
        .route(
            "/api/questions/generate",
            post(handlers::evaluations::generate_test_cases),
        )
        // Datasets
        .route(
            "/api/datasets",
            get(handlers::datasets::list).post(handlers::datasets::create),
        )
        .route("/api/datasets/upload", post(handlers::datasets::upload))
        .route(
            "/api/datasets/:id",
            get(handlers::datasets::get)
                .put(handlers::datasets::update)
                .delete(handlers::datasets::delete),
        )
        .route(
            "/api/datasets/:id/questions",
            get(handlers::datasets::get_questions).post(handlers::datasets::add_question),
        )
        // Stats
        .route("/api/stats", get(handlers::stats::get))
        .with_state(state)
}
