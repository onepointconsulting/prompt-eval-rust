use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;

#[derive(Deserialize, Debug)]
pub struct EvaluationRequest {
    /// Preferred: `datasets.id` from the API (e.g. `ds_…` or UUID).
    #[serde(default)]
    pub dataset_id: Option<String>,
    /// Legacy / fallback: same as id or human-readable dataset `name`.
    #[serde(default)]
    pub dataset_path: Option<String>,
    pub prompt_ids: Vec<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct EvaluationResult {
    pub id: String,
    pub average_score: f64,
    pub total_items: i32,
    pub scores: Vec<f64>,
    pub dataset: String,
    pub prompts: Vec<String>,
    pub created_at: DateTime<Utc>, // ← Changed
}

#[derive(Serialize, Clone, Debug)]
pub struct QuestionDetail {
    pub prompt_id: String,
    pub question: String,
    pub response: String,
    pub score: f64,
    pub strengths: Option<Vec<String>>,
    pub weaknesses: Option<Vec<String>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct EvaluationWithDetails {
    pub id: String,
    pub average_score: f64,
    pub total_items: i32,
    pub scores: Vec<f64>,
    pub dataset: String,
    pub prompts: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub details: Vec<QuestionDetail>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Dataset {
    pub id: String,
    pub name: String,
    pub question_count: i32,
    pub avg_score: Option<f64>,
    pub evaluations: i32,
    pub last_used: Option<String>,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub template: String,
    pub variables: Option<Vec<String>>,
    pub is_templated: bool,
    pub status: String,
    pub runs: i32,
    pub updated_at: DateTime<Utc>,
    pub average_score: Option<f64>,
}

#[derive(Serialize)]
pub struct DashboardStats {
    pub total_evaluations: usize,
    pub active_prompts: usize,
    pub average_score: f64,
    pub success_rate: usize,
}

#[derive(Deserialize)]
pub struct CreateDatasetRequest {
    pub name: String,
    pub question_count: i32,
}

#[derive(Deserialize)]
pub struct DatasetUploadRequest {
    pub name: String,
    pub description: Option<String>,
    pub questions: Vec<QuestionInput>,
}

#[derive(Deserialize, Debug)]
pub struct QuestionInput {
    pub question: String,
    pub answer: Option<String>,
    #[serde(default)]
    pub variable_values: Option<Value>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Question {
    pub id: i32,
    pub dataset_id: String,
    pub question_text: String,
    pub expected_answer: Option<String>,
    pub question_order: i32,
    pub variable_values: Option<Value>,
    pub tags: Option<Vec<String>>,
}

#[derive(Serialize, Clone, Debug)]
pub struct DatasetWithQuestions {
    pub dataset: Dataset,
    pub questions: Vec<Question>,
}

#[derive(Deserialize)]
pub struct CreatePromptRequest {
    pub name: String,
    pub template: String,
    #[serde(default)]
    pub variables: Option<Vec<String>>,
    #[serde(default)]
    pub is_templated: Option<bool>,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePromptRequest {
    pub name: Option<String>,
    pub template: Option<String>,
    pub variables: Option<Vec<String>>,
    pub is_templated: Option<bool>,
    pub status: Option<String>,
}

#[derive(Deserialize)]
pub struct GeneratePromptRequest {
    pub description: String,
}

#[derive(Serialize)]
pub struct GeneratePromptResponse {
    pub template: String,
    pub variables: Vec<String>,
}

#[derive(Deserialize)]
pub struct GenerateTestCasesRequest {
    pub prompt_id: String,
    #[serde(default = "default_test_case_count")]
    pub count: i32,
}

fn default_test_case_count() -> i32 {
    5
}

#[derive(Serialize)]
pub struct GeneratedTestCase {
    pub variable_values: Value,
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct GenerateTestCasesResponse {
    pub test_cases: Vec<GeneratedTestCase>,
}

#[derive(Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
    pub id: String,
}
