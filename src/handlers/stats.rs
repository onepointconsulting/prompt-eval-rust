use crate::models::DashboardStats;
use axum::{extract::State, http::StatusCode, Json};
use sqlx::PgPool;

// GET /api/stats
pub async fn get(State(pool): State<PgPool>) -> Result<Json<DashboardStats>, StatusCode> {
    println!("📊 Getting dashboard stats");

    let total_evaluations: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM evaluation_runs")
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            eprintln!("DB error counting evaluation_runs: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let active_prompts: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM prompts WHERE status = 'active'")
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                eprintln!("DB error counting active prompts: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    let average_score: f64 =
        sqlx::query_scalar("SELECT COALESCE(AVG(average_score), 0.0) FROM evaluation_runs")
            .fetch_one(&pool)
            .await
            .map_err(|e| {
                eprintln!("DB error computing average score: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

    // Success rate: % of runs with average_score >= 7.0
    let success_rate = if total_evaluations == 0 {
        0.0
    } else {
        let successful: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM evaluation_runs WHERE average_score >= 7.0",
        )
        .fetch_one(&pool)
        .await
        .map_err(|e| {
            eprintln!("DB error computing success rate: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        (successful as f64 / total_evaluations as f64) * 100.0
    };

    Ok(Json(DashboardStats {
        total_evaluations,
        active_prompts,
        average_score,
        success_rate,
    }))
}
