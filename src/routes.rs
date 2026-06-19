use crate::app_state::AppState;
use crate::handlers;
use axum::{
    routing::{get, post},
    Router,
};

const API_BASE_URL: &str = "/rust-api";

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Auth (public — no AuthUser extractor)
        .route(format!("{API_BASE_URL}/auth/login").as_str(), post(handlers::auth::login))
        // Evaluations
        .route(format!("{API_BASE_URL}/evaluate").as_str(), post(handlers::evaluations::run_evaluation))
        .route(
            format!("{API_BASE_URL}/evaluations").as_str(),
            get(handlers::evaluations::list_evaluations),
        )
        .route(
            format!("{API_BASE_URL}/evaluations/:id").as_str(),
            get(handlers::evaluations::get_evaluation),
        )
        // Prompts
        .route(format!("{API_BASE_URL}/prompts").as_str(), get(handlers::prompts::list))
        .route(format!("{API_BASE_URL}/prompts").as_str(), post(handlers::prompts::create))
        .route(
            format!("{API_BASE_URL}/prompts/generate").as_str(),
            post(handlers::prompts::generate_prompt),
        )
        .route(
            format!("{API_BASE_URL}/prompts/:id").as_str(),
            get(handlers::prompts::get)
                .put(handlers::prompts::update)
                .delete(handlers::prompts::delete),
        )
        .route(
            format!("{API_BASE_URL}/questions/generate").as_str(),
            post(handlers::evaluations::generate_test_cases),
        )
        // Datasets
        .route(
            format!("{API_BASE_URL}/datasets").as_str(),
            get(handlers::datasets::list).post(handlers::datasets::create),
        )
        .route(format!("{API_BASE_URL}/datasets/upload").as_str(), post(handlers::datasets::upload))
        .route(
            format!("{API_BASE_URL}/datasets/:id").as_str(),
            get(handlers::datasets::get)
                .put(handlers::datasets::update)
                .delete(handlers::datasets::delete),
        )
        .route(
            format!("{API_BASE_URL}/datasets/:id/questions").as_str(),
            get(handlers::datasets::get_questions).post(handlers::datasets::add_question),
        )
        // Stats
        .route(format!("{API_BASE_URL}/stats").as_str(), get(handlers::stats::get))
        .with_state(state)
}
