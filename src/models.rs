use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;

// ── Rubric & Judge Output ─────────────────────────────────────────────────────

/// One criterion in a prompt's evaluation rubric.
/// All weights across a rubric should sum to 1.0.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RubricCriterion {
    pub name: String,
    pub description: String,
    pub weight: f64,
}

/// Score the judge assigned to a single rubric dimension.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DimensionScore {
    pub score: f64,        // 1.0–10.0
    pub reasoning: String, // one-sentence explanation
}

/// Complete structured output returned by the LLM judge for one (question, response) pair.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JudgeOutput {
    /// Per-criterion scores — key is RubricCriterion.name.
    pub dimension_scores: HashMap<String, DimensionScore>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    /// Weighted average of dimension scores (1.0–10.0).
    pub overall_score: f64,
    /// Judge's chain-of-thought reasoning before scores were assigned.
    pub judge_reasoning: String,
    /// True when expected_answer was available and used in the assessment.
    pub reference_used: bool,
}

// ── Evaluations ───────────────────────────────────────────────────────────────

#[derive(Deserialize, Debug)]
pub struct EvaluationRequest {
    /// Preferred: datasets.id (e.g. `ds_1234567890`).
    #[serde(default)]
    pub dataset_id: Option<String>,
    /// Legacy fallback: dataset name used as lookup key.
    #[serde(default)]
    pub dataset_path: Option<String>,
    pub prompt_ids: Vec<String>,
    /// Optional control prompt. Reserved for async evaluation (Step 9).
    #[serde(default)]
    #[allow(dead_code)]
    pub baseline_prompt_id: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct EvaluationResult {
    pub id: String,
    pub status: String,
    pub average_score: f64,
    pub total_items: i32,
    pub scores: Vec<f64>,
    pub dataset: String,
    pub prompts: Vec<String>,
    pub per_prompt_scores: HashMap<String, f64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, Clone, Debug)]
pub struct QuestionDetail {
    pub prompt_id: String,
    pub question: String,
    pub response: String,
    pub score: f64,
    pub strengths: Option<Vec<String>>,
    pub weaknesses: Option<Vec<String>>,
    pub dimension_scores: Option<Value>,
    pub judge_reasoning: Option<String>,
    pub reference_used: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct EvaluationWithDetails {
    pub id: String,
    pub status: String,
    pub average_score: f64,
    pub total_items: i32,
    pub scores: Vec<f64>,
    pub dataset: String,
    pub prompts: Vec<String>,
    pub per_prompt_scores: HashMap<String, f64>,
    pub created_at: DateTime<Utc>,
    pub details: Vec<QuestionDetail>,
}

// ── Datasets & Questions ──────────────────────────────────────────────────────

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

/// Full question row as returned from the DB and the API.
#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Question {
    pub id: i32,
    pub dataset_id: String,
    pub question_text: String,
    pub expected_answer: Option<String>,
    pub question_order: i32,
    pub variable_values: Option<Value>,
    pub tags: Option<Vec<String>>,
    pub difficulty: Option<String>,
    pub case_type: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
pub struct DatasetWithQuestions {
    pub dataset: Dataset,
    pub questions: Vec<Question>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardStats {
    pub total_evaluations: i64,
    pub active_prompts: i64,
    pub average_score: f64,
    /// Percentage of evaluation runs with average_score >= 7.0 (0–100).
    pub success_rate: f64,
}

#[derive(Deserialize)]
pub struct CreateDatasetRequest {
    pub name: String,
    pub question_count: i32,
}

#[derive(Deserialize)]
pub struct UpdateDatasetRequest {
    pub name: Option<String>,
}

#[derive(Deserialize)]
pub struct DatasetUploadRequest {
    pub name: String,
    pub description: Option<String>,
    pub questions: Vec<QuestionInput>,
}

/// Input shape for creating or uploading a question.
#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct QuestionInput {
    pub question: String,
    pub answer: Option<String>,
    #[serde(default)]
    pub variable_values: Option<Value>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub difficulty: Option<String>,
    #[serde(default)]
    pub case_type: Option<String>,
    /// Why this test case probes what it probes (generated by AI, stored for reference).
    #[serde(default)]
    pub reasoning: Option<String>,
}

// ── Prompts ───────────────────────────────────────────────────────────────────

/// Full prompt row as returned from the DB and the API.
#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct Prompt {
    pub id: String,
    pub name: String,
    pub template: String,
    pub variables: Option<Vec<String>>,
    pub is_templated: bool,
    pub status: String, // draft | active | archived
    pub runs: i32,
    pub updated_at: DateTime<Utc>,
    pub average_score: Option<f64>,
    pub domain: Option<String>,
    pub rubric: Option<Value>,
    pub expected_output_format: Option<String>,
    pub use_context: bool,
    pub context_project: Option<String>,
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
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub rubric: Option<Value>,
    #[serde(default)]
    pub expected_output_format: Option<String>,
    #[serde(default)]
    pub use_context: Option<bool>,
    #[serde(default)]
    pub context_project: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdatePromptRequest {
    pub name: Option<String>,
    pub template: Option<String>,
    pub variables: Option<Vec<String>>,
    pub is_templated: Option<bool>,
    pub status: Option<String>,
    pub domain: Option<String>,
    pub rubric: Option<Value>,
    pub expected_output_format: Option<String>,
    pub use_context: Option<bool>,
    pub context_project: Option<String>,
}

#[derive(Deserialize)]
pub struct GeneratePromptRequest {
    pub description: String,
    #[serde(default)]
    pub existing_template: Option<String>,
}

/// Response from POST /api/prompts/generate.
#[derive(Serialize)]
pub struct GeneratePromptResponse {
    pub template: String,
    pub variables: Vec<String>,
    pub domain: String,
    pub rubric: Vec<RubricCriterion>,
    pub expected_output_format: String,
}

// ── Test Case Generation ──────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GenerateTestCasesRequest {
    pub prompt_id: String,
    #[serde(default = "default_test_case_count")]
    pub count: i32,
}

fn default_test_case_count() -> i32 {
    5
}

/// One AI-generated test case with rich metadata.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GeneratedTestCase {
    pub variable_values: Value,
    pub expected_answer: Option<String>,
    pub difficulty: String,
    pub case_type: String,
    pub tags: Vec<String>,
    pub reasoning: String,
}

#[derive(Serialize)]
pub struct GenerateTestCasesResponse {
    pub test_cases: Vec<GeneratedTestCase>,
}

// ── Misc ──────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct DeleteResponse {
    pub deleted: bool,
    pub id: String,
}
