use crate::models::{
    CreatePromptRequest, DeleteResponse, GeneratePromptRequest, GeneratePromptResponse, Prompt,
    UpdatePromptRequest,
};
use crate::utils::template_parser::extract_variables;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use sqlx::PgPool;
use std::env;

// GET /api/prompts
pub async fn list(State(pool): State<PgPool>) -> Result<Json<Vec<Prompt>>, StatusCode> {
    println!("📋 Listing prompts from database");

    let prompts = sqlx::query_as::<_, Prompt>(
        r#"
        SELECT id, name, template, variables, is_templated, status, runs, updated_at, average_score
        FROM prompts
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(prompts))
}

// GET /api/prompts/:id
pub async fn get(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<Prompt>, StatusCode> {
    println!("🔎 Fetching prompt {}", id);

    let prompt = sqlx::query_as::<_, Prompt>(
        r#"
        SELECT id, name, template, variables, is_templated, status, runs, updated_at, average_score
        FROM prompts WHERE id = $1
        "#,
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(prompt))
}

// POST /api/prompts
pub async fn create(
    State(pool): State<PgPool>,
    Json(payload): Json<CreatePromptRequest>,
) -> Result<(StatusCode, Json<Prompt>), StatusCode> {
    println!("➕ Creating prompt: {}", payload.name);

    let id = format!("p_{}", Utc::now().timestamp());
    let now = Utc::now(); // ← Use DateTime instead of String
    let variables = payload
        .variables
        .unwrap_or_else(|| extract_variables(&payload.template));
    let is_templated = payload.is_templated.unwrap_or(!variables.is_empty());
    let status = payload.status.unwrap_or_else(|| "draft".to_string());

    let prompt = sqlx::query_as::<_, Prompt>(
        r#"
        INSERT INTO prompts (id, name, template, variables, is_templated, status, runs, updated_at, average_score)
        VALUES ($1, $2, $3, $4, $5, $6, 0, $7, NULL)
        RETURNING id, name, template, variables, is_templated, status, runs, updated_at, average_score
        "#,
    )
    .bind(id)
    .bind(payload.name)
    .bind(payload.template)
    .bind(variables)
    .bind(is_templated)
    .bind(status)
    .bind(now)
    .fetch_one(&pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
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

    // Get existing prompt
    let existing = sqlx::query_as::<_, Prompt>(
        r#"
        SELECT id, name, template, variables, is_templated, status, runs, updated_at, average_score
        FROM prompts WHERE id = $1
        "#,
    )
    .bind(&id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Handle Option<String> fields
    let new_name = match payload.name {
        Some(n) => n,
        None => existing.name.clone(),
    };

    let new_template = match payload.template {
        Some(t) => t,
        None => existing.template.clone(),
    };

    let new_status = match payload.status {
        Some(s) => s,
        None => existing.status.clone(),
    };
    let new_variables = payload
        .variables
        .or(existing.variables.clone())
        .unwrap_or_else(|| extract_variables(&new_template));
    let new_is_templated = payload
        .is_templated
        .unwrap_or(existing.is_templated || !new_variables.is_empty());

    let now = Utc::now(); // ← DateTime type

    let updated = sqlx::query_as::<_, Prompt>(
        r#"
        UPDATE prompts
        SET name = $1, template = $2, variables = $3, is_templated = $4, status = $5, updated_at = $6
        WHERE id = $7
        RETURNING id, name, template, variables, is_templated, status, runs, updated_at, average_score
        "#,
    )
    .bind(new_name)
    .bind(new_template)
    .bind(new_variables)
    .bind(new_is_templated)
    .bind(new_status)
    .bind(now)
    .bind(id)
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

// POST /api/prompts/generate
pub async fn generate_prompt(
    Json(payload): Json<GeneratePromptRequest>,
) -> Result<Json<GeneratePromptResponse>, StatusCode> {
    let description = payload.description.trim();
    if description.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let generated_template = generate_template_with_ai(description)
        .await
        .unwrap_or_else(|_| {
            format!(
                "You are a helpful assistant.\nContext: {{{{CONTEXT}}}}\nUser question: {{{{QUESTION}}}}\nTask: {}",
                description
            )
        });

    let variables = extract_variables(&generated_template);

    Ok(Json(GeneratePromptResponse {
        template: generated_template,
        variables,
    }))
}

async fn generate_template_with_ai(description: &str) -> Result<String, StatusCode> {
    let api_key = env::var("ANTHROPIC_API_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let client = Client::new();

    let request_body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1024,
        "messages": [{
            "role": "user",
            "content": format!(
                "Generate a reusable prompt template for this description: {}\nUse placeholders in {{VAR_NAME}} format (double braces). Include at least {{QUESTION}} when relevant. Return only the template text.",
                description
            )
        }]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body)
        .send()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let data: serde_json::Value = response
        .json()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let template = data["content"][0]["text"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(template.to_string())
}
