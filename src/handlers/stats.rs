use crate::models::DashboardStats;
use axum::Json;

// GET /api/stats
pub async fn get() -> Json<DashboardStats> {
    println!("📊 Getting dashboard stats");

    Json(DashboardStats {
        total_evaluations: 124,
        active_prompts: 12,
        average_score: 8.5,
        success_rate: 95,
    })
}
