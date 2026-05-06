use crate::models::{
    Dataset, EvaluationRequest, EvaluationResult, EvaluationWithDetails, GenerateTestCasesRequest,
    GenerateTestCasesResponse, GeneratedTestCase, QuestionDetail,
};
use crate::utils::template_parser::{extract_variables, fill_template};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use reqwest::Client;
use serde_json::json;
use sqlx::PgPool;
use std::env;

fn preview(input: &str, max_chars: usize) -> String {
    let compact = input.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        format!("{}...", compact.chars().take(max_chars).collect::<String>())
    }
}

async fn fetch_dataset_by_id(pool: &PgPool, id: &str) -> Result<Dataset, StatusCode> {
    sqlx::query_as::<_, Dataset>(
        r#"
        SELECT
            id,
            name,
            question_count,
            avg_score,
            evaluations,
            NULL::text as last_used,
            created_at::text as created_at
        FROM datasets
        WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("Database error resolving dataset id: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)
}

/// Match by primary key first; if missing, fall back to unique name (legacy `dataset_path`).
async fn fetch_dataset_by_id_or_name(pool: &PgPool, key: &str) -> Result<Dataset, StatusCode> {
    sqlx::query_as::<_, Dataset>(
        r#"
        SELECT
            id,
            name,
            question_count,
            avg_score,
            evaluations,
            NULL::text as last_used,
            created_at::text as created_at
        FROM datasets
        WHERE id = $1 OR name = $1
        LIMIT 1
        "#,
    )
    .bind(key)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("Database error resolving dataset: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)
}

async fn resolve_dataset(
    pool: &PgPool,
    payload: &EvaluationRequest,
) -> Result<Dataset, StatusCode> {
    if let Some(id) = payload.dataset_id.as_ref().filter(|s| !s.is_empty()) {
        return fetch_dataset_by_id(pool, id).await;
    }
    if let Some(path) = payload.dataset_path.as_ref().filter(|s| !s.is_empty()) {
        return fetch_dataset_by_id_or_name(pool, path).await;
    }
    Err(StatusCode::BAD_REQUEST)
}

#[derive(Clone)]
struct EvalQuestion {
    id: i32,
    question_text: String,
    variable_values: Option<serde_json::Value>,
}

async fn load_questions_with_ids(
    pool: &PgPool,
    dataset_id: &str,
) -> Result<Vec<EvalQuestion>, StatusCode> {
    let rows = sqlx::query!(
        r#"
        SELECT id, question_text, variable_values
        FROM questions
        WHERE dataset_id = $1
        ORDER BY question_order
        "#,
        dataset_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        eprintln!("Database error loading questions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(rows
        .into_iter()
        .map(|r| EvalQuestion {
            id: r.id,
            question_text: r.question_text,
            variable_values: r.variable_values,
        })
        .collect())
}

// Your existing evaluation logic
async fn ask_claude(
    client: &Client,
    api_key: &str,
    prompt: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let request_body = json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 1000,
        "messages": [{
            "role": "user",
            "content": prompt
        }]
    });

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body)
        .send()
        .await?;

    let body_text = response.text().await?;

    // Simple parsing - just get the text
    Ok(body_text)
}

pub async fn run_evaluation(
    State(pool): State<PgPool>,
    Json(payload): Json<EvaluationRequest>,
) -> Result<Json<EvaluationResult>, StatusCode> {
    println!("📝 Starting evaluation...");
    println!("   request.dataset_id={:?}", payload.dataset_id);
    println!("   request.dataset_path={:?}", payload.dataset_path);
    println!("   request.prompt_ids={:?}", payload.prompt_ids);

    let dataset = resolve_dataset(&pool, &payload).await?;
    let questions_with_ids = load_questions_with_ids(&pool, &dataset.id).await?;
    if questions_with_ids.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    println!(
        "   resolved.dataset_id={} dataset_name=\"{}\" questions={}",
        dataset.id,
        dataset.name,
        questions_with_ids.len()
    );

    let api_key = env::var("ANTHROPIC_API_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let client = Client::new();

    // Create the main evaluation run record
    let run_id = format!("eval_{}", chrono::Utc::now().timestamp());
    let created_at = chrono::Utc::now();

    let mut all_scores = vec![];
    let mut all_details = vec![]; // Store for batch insert

    // For EACH prompt template
    for prompt_id in &payload.prompt_ids {
        println!("\n🔄 Testing prompt_id={}", prompt_id);

        let prompt = load_prompt_template(&pool, prompt_id).await?;
        println!(
            "   prompt.is_templated={} prompt.variables={:?}",
            prompt.is_templated, prompt.variables
        );
        println!("   prompt.template_preview=\"{}\"", preview(&prompt.template, 160));
        let mut run_scores = vec![];

        // Test this prompt against all questions
        for (i, question) in questions_with_ids.iter().enumerate() {
            println!(
                "   Q{} question_id={} text=\"{}\"",
                i + 1,
                question.id,
                preview(&question.question_text, 120)
            );
            println!(
                "      variable_values_present={} tags_or_vars_preview={}",
                question.variable_values.is_some(),
                question
                    .variable_values
                    .as_ref()
                    .map(|v| preview(&v.to_string(), 160))
                    .unwrap_or_else(|| "-".to_string())
            );

            let mut full_prompt = if prompt.is_templated {
                if let Some(var_values) = &question.variable_values {
                    fill_template(&prompt.template, var_values)
                } else {
                    let fallback = json!({
                        "QUESTION": question.question_text
                    });
                    fill_template(&prompt.template, &fallback)
                }
            } else {
                format!(
                    "{}\n\nUser question: {}",
                    prompt.template, question.question_text
                )
            };
            // Always append the plain question to preserve direct user intent,
            // even when a rich templated variable payload is provided.
            full_prompt.push_str("\n\n---\n\n");
            full_prompt.push_str("**Student asks:** ");
            full_prompt.push_str(&question.question_text);
            full_prompt.push_str("\n\n**Your response:**");
            println!("      full_prompt_preview=\"{}\"", preview(&full_prompt, 220));

            // Get response from Claude
            let response = ask_claude(&client, &api_key, &full_prompt)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            println!(
                "   ✅ Response received (chars={}) preview=\"{}\"",
                response.chars().count(),
                preview(&response, 220)
            );

            // JUDGE the response quality
            let score = judge_response(&client, &api_key, &question.question_text, &response)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            println!("   📊 Score for question_id={} => {:.1}/10", question.id, score);

            run_scores.push(score);
            all_scores.push(score);

            // Store detail for later insert
            all_details.push(EvaluationDetail {
                run_id: run_id.clone(),
                prompt_id: prompt_id.clone(),
                question_id: question.id,
                response: response.clone(),
                score,
            });
        }

        let run_avg = run_scores.iter().sum::<f64>() / run_scores.len() as f64;
        println!(
            "   prompt_id={} run_scores={:?} run_average={:.3}",
            prompt_id, run_scores, run_avg
        );

        // Update prompt stats
        persist_prompt_eval_stats(&pool, prompt_id, run_avg).await?;
        println!("   💾 Updated prompt stats (runs, avg_score)");
    }

    let average = all_scores.iter().sum::<f64>() / all_scores.len() as f64;
    println!(
        "📦 run_id={} total_scored_items={} overall_scores={:?}",
        run_id,
        all_scores.len(),
        all_scores
    );

    // Save the evaluation run to database
    save_evaluation_run(
        &pool,
        &run_id,
        &dataset.id,
        &payload.prompt_ids,
        average,
        questions_with_ids.len() as i32,
        &all_scores,
        created_at,
    )
    .await?;

    // Save all evaluation details
    save_evaluation_details(&pool, &all_details).await?;

    println!(
        "✨ Evaluation complete! run_id={} overall_average={:.3}",
        run_id, average
    );
    println!(
        "💾 Saved run + details (details_count={})",
        all_details.len()
    );

    Ok(Json(EvaluationResult {
        id: run_id,
        average_score: average,
        total_items: all_scores.len() as i32,
        scores: all_scores,
        dataset: dataset.name,
        prompts: payload.prompt_ids,
        created_at,
    }))
}

// Helper struct for batch insert
struct EvaluationDetail {
    run_id: String,
    prompt_id: String,
    question_id: i32,
    response: String,
    score: f64,
}

// Save main evaluation run
async fn save_evaluation_run(
    pool: &PgPool,
    run_id: &str,
    dataset_id: &str,
    prompt_ids: &[String],
    average_score: f64,
    total_questions: i32,
    _scores: &[f64],
    created_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), StatusCode> {
    println!(
        "   [db] insert evaluation_runs id={} dataset_id={} prompts={} total_questions={} average={:.3}",
        run_id,
        dataset_id,
        prompt_ids.len(),
        total_questions,
        average_score
    );
    sqlx::query!(
        r#"
        INSERT INTO evaluation_runs 
            (id, dataset_id, prompt_ids, average_score, total_questions, status, created_at)
        VALUES ($1, $2, $3, $4, $5, 'completed', $6)
        "#,
        run_id,
        dataset_id,
        prompt_ids,
        average_score,
        total_questions,
        created_at
    )
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("Failed to save evaluation run: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

// Save individual question results
async fn save_evaluation_details(
    pool: &PgPool,
    details: &[EvaluationDetail],
) -> Result<(), StatusCode> {
    // Batch insert all details
    println!("   [db] inserting evaluation_details rows={}", details.len());
    for detail in details {
        println!(
            "      [db] detail run_id={} prompt_id={} question_id={} score={:.2} response_chars={}",
            detail.run_id,
            detail.prompt_id,
            detail.question_id,
            detail.score,
            detail.response.chars().count()
        );
        sqlx::query!(
            r#"
            INSERT INTO evaluation_details 
                (run_id, question_id, prompt_id, model_answer, score, created_at)
            VALUES ($1, $2, $3, $4, $5, NOW())
            "#,
            detail.run_id,
            detail.question_id,
            detail.prompt_id,
            detail.response,
            detail.score
        )
        .execute(pool)
        .await
        .map_err(|e| {
            eprintln!("Failed to save evaluation detail: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    Ok(())
}

/// Increment `runs` and merge `average_score` (rolling mean across evaluation runs).
async fn persist_prompt_eval_stats(
    pool: &PgPool,
    prompt_id: &str,
    this_run_average: f64,
) -> Result<(), StatusCode> {
    let res = sqlx::query(
        r#"
        UPDATE prompts
        SET
            runs = runs + 1,
            average_score = CASE
                WHEN runs = 0 THEN $1
                ELSE (COALESCE(average_score, 0.0) * runs::float8 + $1)
                    / ((runs + 1)::float8)
            END,
            updated_at = NOW()
        WHERE id = $2
        "#,
    )
    .bind(this_run_average)
    .bind(prompt_id)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("❌ Failed to update prompt stats for {}: {}", prompt_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if res.rows_affected() == 0 {
        eprintln!("❌ No row updated for prompt_id={}", prompt_id);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(())
}

// Load prompt template from database (404 here looked like "route not found" in the browser).
struct LoadedPrompt {
    template: String,
    variables: Vec<String>,
    is_templated: bool,
}

async fn load_prompt_template(pool: &PgPool, prompt_id: &str) -> Result<LoadedPrompt, StatusCode> {
    let row = sqlx::query!(
        "SELECT template, variables, is_templated FROM prompts WHERE id = $1",
        prompt_id
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or_else(|| {
        eprintln!("❌ Unknown prompt id (not in DB): {prompt_id}");
        StatusCode::BAD_REQUEST
    })?;

    let inferred_variables = row
        .variables
        .unwrap_or_else(|| extract_variables(&row.template));
    let is_templated = row.is_templated.unwrap_or(!inferred_variables.is_empty());

    Ok(LoadedPrompt {
        template: row.template,
        variables: inferred_variables,
        is_templated,
    })
}

// NEW: Judge response quality with LLM
async fn judge_response(
    client: &Client,
    api_key: &str,
    question: &str,
    response: &str,
) -> Result<f64, Box<dyn std::error::Error>> {
    let judge_prompt = format!(
        r#"You are evaluating a customer support response. Rate it on a scale of 1-10.

Question: {}

Response: {}

Criteria:
- Did it answer the question directly?
- Was it helpful and professional?
- Did it provide clear next steps?
- Did it avoid making false promises?

Respond with ONLY a number between 1-10, nothing else."#,
        question, response
    );

    let request_body = json!({
        "model": "claude-sonnet-4-20250514",
        "max_tokens": 10,
        "messages": [{
            "role": "user",
            "content": judge_prompt
        }]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request_body)
        .send()
        .await?;

    let data: serde_json::Value = resp.json().await?;

    // Extract score from response
    let score_text = data["content"][0]["text"].as_str().unwrap_or("5.0");

    let score: f64 = score_text.trim().parse().unwrap_or(5.0);

    Ok(score.max(1.0).min(10.0))
}

// GET /api/evaluations - List all evaluation runs
pub async fn list_evaluations(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<EvaluationResult>>, StatusCode> {
    println!("📋 Listing evaluations from database");

    // Single query with JOIN - gets everything at once
    let rows = sqlx::query!(
        r#"
        SELECT 
            er.id,
            er.prompt_ids,
            er.average_score,
            er.total_questions,
            er.created_at,
            d.name as dataset_name,
            array_agg(ed.score ORDER BY ed.id) as scores
        FROM evaluation_runs er
        LEFT JOIN datasets d ON er.dataset_id = d.id
        LEFT JOIN evaluation_details ed ON er.id = ed.run_id
        GROUP BY er.id, er.prompt_ids, er.average_score, er.total_questions, er.created_at, d.name
        ORDER BY er.created_at DESC
        LIMIT 50
        "#
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("Database error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let results = rows
        .into_iter()
        .map(|row| EvaluationResult {
            id: row.id,
            average_score: row.average_score,
            total_items: row.total_questions,
            scores: row.scores.unwrap_or_default(),
            dataset: row.dataset_name,
            prompts: row.prompt_ids,
            created_at: row.created_at.unwrap(),
        })
        .collect();

    Ok(Json(results))
}

// GET /api/evaluations/:id - Get specific evaluation with details
pub async fn get_evaluation(
    State(pool): State<PgPool>,
    Path(id): Path<String>,
) -> Result<Json<EvaluationWithDetails>, StatusCode> {
    println!("🔍 Getting evaluation: {}", id);

    // Get main run info
    let run = sqlx::query!(
        r#"
        SELECT 
            id,
            dataset_id,
            prompt_ids,
            average_score,
            total_questions,
            status,
            created_at
        FROM evaluation_runs
        WHERE id = $1
        "#,
        id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Get dataset name
    let dataset_id = run
        .dataset_id
        .clone()
        .unwrap_or_else(|| "unknown_dataset".to_string());

    let dataset_name = sqlx::query_scalar::<_, String>("SELECT name FROM datasets WHERE id = $1")
        .bind(&dataset_id)
        .fetch_optional(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .unwrap_or_else(|| dataset_id.clone());

    // Get all evaluation details with question text
    let details = sqlx::query!(
        r#"
        SELECT 
            ed.prompt_id,
            q.question_text,
            ed.model_answer,
            ed.score,
            ed.strengths,
            ed.weaknesses
        FROM evaluation_details ed
        JOIN questions q ON ed.question_id = q.id
        WHERE ed.run_id = $1
        ORDER BY ed.id
        "#,
        id
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let scores: Vec<f64> = details.iter().filter_map(|d| d.score).collect();

    let created_at = run.created_at.unwrap_or_else(chrono::Utc::now);

    Ok(Json(EvaluationWithDetails {
        id: run.id,
        average_score: run.average_score,
        total_items: run.total_questions,
        scores,
        dataset: dataset_name,
        prompts: run.prompt_ids,
        created_at,
        details: details
            .into_iter()
            .map(|d| QuestionDetail {
                prompt_id: d.prompt_id.unwrap_or_default(),
                question: d.question_text,
                response: d.model_answer.unwrap_or_default(),
                score: d.score.unwrap_or(0.0),
                strengths: d.strengths,
                weaknesses: d.weaknesses,
            })
            .collect(),
    }))
}

// POST /api/questions/generate
pub async fn generate_test_cases(
    State(pool): State<PgPool>,
    Json(payload): Json<GenerateTestCasesRequest>,
) -> Result<Json<GenerateTestCasesResponse>, StatusCode> {
    let count = payload.count.clamp(1, 20);
    let prompt = load_prompt_template(&pool, &payload.prompt_id).await?;
    let cases = generate_test_cases_with_ai(&prompt.template, &prompt.variables, count).await?;
    Ok(Json(GenerateTestCasesResponse { test_cases: cases }))
}

async fn generate_test_cases_with_ai(
    template: &str,
    variables: &[String],
    count: i32,
) -> Result<Vec<GeneratedTestCase>, StatusCode> {
    let api_key = env::var("ANTHROPIC_API_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let client = Client::new();

    let make_prompt = |retry: bool| {
        if retry {
            format!(
                "Return ONLY minified JSON (no markdown, no prose). Output exactly {} items.\nSchema: [{{\"variable_values\":{{...}},\"tags\":[\"tag1\",\"tag2\"]}}]\nAll string values must be short (<= 80 chars). Use variables {:?}.",
                count, variables
            )
        } else {
            format!(
                "Given this prompt template:\n{}\n\nVariables: {:?}\n\nGenerate {} diverse test cases as a JSON array.\nRules:\n- Return ONLY valid JSON array.\n- No markdown fences.\n- Keep each string concise (<80 chars) to avoid truncation.\n- Each item shape: {{\"variable_values\":{{...}},\"tags\":[\"...\"]}}.",
                template, variables, count
            )
        }
    };

    let mut parsed: Option<serde_json::Value> = None;
    for retry in [false, true] {
        let request_body = json!({
            "model": "claude-haiku-4-5-20251001",
            "max_tokens": 3000,
            "messages": [{
                "role": "user",
                "content": make_prompt(retry)
            }]
        });

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request_body)
            .send()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !status.is_success() {
            eprintln!("generate_test_cases anthropic non-2xx: {}", body_text);
            return Err(StatusCode::BAD_GATEWAY);
        }

        let data: serde_json::Value =
            serde_json::from_str(&body_text).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let text = data["content"][0]["text"]
            .as_str()
            .ok_or(StatusCode::BAD_GATEWAY)?;

        // Be lenient with fenced JSON.
        let cleaned = text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        let try_parsed = serde_json::from_str::<serde_json::Value>(cleaned).or_else(|_| {
            let start = cleaned.find('[').ok_or(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no array start",
            )))?;
            let end = cleaned.rfind(']').ok_or(serde_json::Error::io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no array end",
            )))?;
            serde_json::from_str(&cleaned[start..=end])
        });

        match try_parsed {
            Ok(v) => {
                parsed = Some(v);
                break;
            }
            Err(e) => {
                eprintln!("generate_test_cases parse error: {e}; raw={}", cleaned);
            }
        }
    }

    let parsed = parsed.ok_or(StatusCode::BAD_GATEWAY)?;
    let arr = if let Some(a) = parsed.as_array() {
        a
    } else if let Some(a) = parsed.get("test_cases").and_then(|v| v.as_array()) {
        a
    } else {
        eprintln!("generate_test_cases unexpected JSON shape: {}", parsed);
        return Err(StatusCode::BAD_GATEWAY);
    };

    let test_cases = arr
        .iter()
        .take(count as usize)
        .map(|v| GeneratedTestCase {
            variable_values: v
                .get("variable_values")
                .cloned()
                .unwrap_or_else(|| json!({ "QUESTION": "Sample question" })),
            tags: v
                .get("tags")
                .and_then(|t| t.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect();

    Ok(test_cases)
}
