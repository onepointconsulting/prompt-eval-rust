use crate::llm::anthropic_client::AnthropicClient;
use crate::models::{
    CreatePromptRequest, DeleteResponse, GeneratePromptRequest, GeneratePromptResponse, Prompt,
    RubricCriterion, UpdatePromptRequest,
};
use crate::utils::template_parser::extract_variables;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;

/// Generic four-dimension rubric used when a prompt has no domain-specific rubric.
/// Weights sum to 1.0, no domain assumptions.
fn default_rubric() -> Vec<RubricCriterion> {
    vec![
        RubricCriterion {
            name: "relevance".to_string(),
            description: "Does the response directly address what was asked?".to_string(),
            weight: 0.25,
        },
        RubricCriterion {
            name: "accuracy".to_string(),
            description: "Are the claims and information factually correct?".to_string(),
            weight: 0.25,
        },
        RubricCriterion {
            name: "completeness".to_string(),
            description: "Does it cover all aspects of the question?".to_string(),
            weight: 0.25,
        },
        RubricCriterion {
            name: "clarity".to_string(),
            description: "Is it well-structured and easy to understand?".to_string(),
            weight: 0.25,
        },
    ]
}

// ── Read handlers (DB only) ───────────────────────────────────────────────────

// GET /api/prompts
pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Prompt>>, StatusCode> {
    let prompts = sqlx::query_as::<_, Prompt>(
        r#"
        SELECT id, name, template, variables, is_templated, status, runs,
               updated_at, average_score, domain, rubric, expected_output_format
        FROM prompts
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("DB error listing prompts: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(prompts))
}

// GET /api/prompts/:id
pub async fn get(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<Prompt>, StatusCode> {
    let prompt = sqlx::query_as::<_, Prompt>(
        r#"
        SELECT id, name, template, variables, is_templated, status, runs,
               updated_at, average_score, domain, rubric, expected_output_format
        FROM prompts WHERE id = $1
        "#,
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        eprintln!("DB error fetching prompt {}: {}", id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(prompt))
}

// ── Write handlers ────────────────────────────────────────────────────────────

// POST /api/prompts
pub async fn create(
    State(pool): State<PgPool>,
    Json(payload): Json<CreatePromptRequest>,
) -> Result<(StatusCode, Json<Prompt>), StatusCode> {
    println!("➕ Creating prompt: {}", payload.name);

    let id = format!("p_{}", Utc::now().timestamp());
    let now = Utc::now();
    let variables = payload
        .variables
        .unwrap_or_else(|| extract_variables(&payload.template));
    let is_templated = payload.is_templated.unwrap_or(!variables.is_empty());
    let status = payload.status.unwrap_or_else(|| "draft".to_string());

    let prompt = sqlx::query_as::<_, Prompt>(
        r#"
        INSERT INTO prompts
            (id, name, template, variables, is_templated, status, runs,
             updated_at, average_score, domain, rubric, expected_output_format)
        VALUES ($1, $2, $3, $4, $5, $6, 0, $7, NULL, $8, $9, $10)
        RETURNING id, name, template, variables, is_templated, status, runs,
                  updated_at, average_score, domain, rubric, expected_output_format
        "#,
    )
    .bind(&id)
    .bind(&payload.name)
    .bind(&payload.template)
    .bind(&variables)
    .bind(is_templated)
    .bind(&status)
    .bind(now)
    .bind(&payload.domain)
    .bind(&payload.rubric)
    .bind(&payload.expected_output_format)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        eprintln!("DB error creating prompt: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((StatusCode::CREATED, Json(prompt)))
}

// PUT /api/prompts/:id
pub async fn update(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
    Json(payload): Json<UpdatePromptRequest>,
) -> Result<Json<Prompt>, StatusCode> {
    println!("✏️  Updating prompt: {}", id);

    let existing = sqlx::query_as::<_, Prompt>(
        r#"
        SELECT id, name, template, variables, is_templated, status, runs,
               updated_at, average_score, domain, rubric, expected_output_format
        FROM prompts WHERE id = $1
        "#,
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let new_name = payload.name.unwrap_or(existing.name);
    let new_template = payload.template.unwrap_or(existing.template.clone());
    let new_status = payload.status.unwrap_or(existing.status);
    let new_variables = payload
        .variables
        .or(existing.variables)
        .unwrap_or_else(|| extract_variables(&new_template));
    let new_is_templated = payload
        .is_templated
        .unwrap_or(existing.is_templated || !new_variables.is_empty());
    let new_domain = payload.domain.or(existing.domain);
    let new_rubric: Option<Value> = payload.rubric.or(existing.rubric);
    let new_output_format = payload.expected_output_format.or(existing.expected_output_format);
    let now = Utc::now();

    let updated = sqlx::query_as::<_, Prompt>(
        r#"
        UPDATE prompts
        SET name = $1, template = $2, variables = $3, is_templated = $4,
            status = $5, updated_at = $6, domain = $7, rubric = $8,
            expected_output_format = $9
        WHERE id = $10
        RETURNING id, name, template, variables, is_templated, status, runs,
                  updated_at, average_score, domain, rubric, expected_output_format
        "#,
    )
    .bind(new_name)
    .bind(new_template)
    .bind(new_variables)
    .bind(new_is_templated)
    .bind(new_status)
    .bind(now)
    .bind(new_domain)
    .bind(new_rubric)
    .bind(new_output_format)
    .bind(&id)
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(updated))
}

// DELETE /api/prompts/:id
pub async fn delete(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<DeleteResponse>, StatusCode> {
    println!("🗑️  Deleting prompt: {}", id);

    let result = sqlx::query("DELETE FROM prompts WHERE id = $1")
        .bind(&id)
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(DeleteResponse { deleted: true, id }))
}

// ── AI generation ─────────────────────────────────────────────────────────────

/// POST /api/prompts/generate
///
/// Uses Sonnet (not Haiku) because template quality is the root of the evaluation
/// chain — a weak template poisons every downstream result. The model returns
/// structured JSON containing the template, rubric, domain, and output format.
/// The frontend should persist the response via POST /api/prompts.
pub async fn generate_prompt(
    State(llm): State<AnthropicClient>,
    Json(payload): Json<GeneratePromptRequest>,
) -> Result<Json<GeneratePromptResponse>, StatusCode> {
    let description = payload.description.trim();
    if description.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let system = "You are an expert prompt engineer. \
                  Generate high-quality, structured prompt templates for LLM evaluation systems. \
                  Always respond with valid JSON only — no markdown, no prose.";

    let user_prompt = format!(
        r#"Design a production-quality reusable prompt template for this task:

{}

Return ONLY valid JSON (no markdown fences, no text before or after):
{{
  "template": "<the full prompt with {{{{VAR_NAME}}}} double-brace placeholders>",
  "variables": [
    {{"name": "VAR_NAME", "description": "what this variable contains and its expected format"}}
  ],
  "domain": "<short snake_case label e.g. educational_assistant, code_review, customer_support>",
  "rubric": [
    {{"name": "criterion_name", "description": "what to assess", "weight": 0.25}}
  ],
  "expected_output_format": "<description of what ideal output looks like>"
}}

Requirements:
- Use {{{{DOUBLE_BRACES}}}} for all template variables — single braces are not valid
- Rubric weights must sum exactly to 1.0
- Include 3-5 rubric criteria specific to this task's goals (not generic)
- Template must include: role definition, clear task description, variable placeholders, output guidance
- The variables list must match exactly the {{{{VAR}}}} placeholders in the template"#,
        description
    );

    let text = llm
        .send_text(llm.model_sonnet(), 2000, &user_prompt, Some(system))
        .await
        .unwrap_or_default();

    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json: Value = serde_json::from_str(cleaned).unwrap_or_else(|e| {
        eprintln!("generate_prompt JSON parse error: {e}\nraw={}", cleaned);
        Value::Null
    });

    let template = json
        .get("template")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string();

    if template.is_empty() {
        // Graceful fallback — never return an error for generation failures.
        let fallback = format!(
            "You are a helpful assistant.\n\
             Context: {{{{CONTEXT}}}}\n\
             User question: {{{{QUESTION}}}}\n\
             Task: {}",
            description
        );
        let variables = extract_variables(&fallback);
        return Ok(Json(GeneratePromptResponse {
            template: fallback,
            variables,
            domain: "general".to_string(),
            rubric: default_rubric(),
            expected_output_format: "Clear, accurate, and helpful response.".to_string(),
        }));
    }

    // Use variables extracted from the template text as ground truth
    // (template is authoritative over the declared list).
    let variables = extract_variables(&template);

    let rubric: Vec<RubricCriterion> = json
        .get("rubric")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .filter(|r: &Vec<RubricCriterion>| !r.is_empty())
        .unwrap_or_else(default_rubric);

    let domain = json
        .get("domain")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("general")
        .to_string();

    let expected_output_format = json
        .get("expected_output_format")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Clear, accurate, and helpful response.")
        .to_string();

    println!(
        "✅ Generated prompt: domain={} rubric_criteria={} variables={:?}",
        domain,
        rubric.len(),
        variables
    );

    Ok(Json(GeneratePromptResponse {
        template,
        variables,
        domain,
        rubric,
        expected_output_format,
    }))
}
