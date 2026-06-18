use crate::auth::AuthUser;
use crate::llm::anthropic_client::AnthropicClient;
use crate::llm::context_client::ContextClient;
use crate::models::{
    Dataset, DimensionScore, EvaluationRequest, EvaluationResult, EvaluationWithDetails,
    GenerateTestCasesRequest, GenerateTestCasesResponse, GeneratedTestCase, JudgeOutput,
    QuestionDetail, RubricCriterion,
};
use crate::utils::template_parser::{extract_variables, fill_template};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use futures::stream::{self, StreamExt};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::collections::HashMap;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn preview(input: &str, max_chars: usize) -> String {
    let compact = input.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        format!("{}...", compact.chars().take(max_chars).collect::<String>())
    }
}

/// Build the criteria block injected into the judge prompt.
/// Falls back to a generic four-dimension rubric when the prompt has none,
/// so the judge is never customer-support-specific by default.
fn build_criteria_text(criteria: &[RubricCriterion]) -> String {
    if criteria.is_empty() {
        return "\
- relevance (weight: 25%): Does the response directly address what was asked?\n\
- accuracy (weight: 25%): Are the claims and information factually correct?\n\
- completeness (weight: 25%): Does it cover all aspects of the question?\n\
- clarity (weight: 25%): Is it well-structured and easy to understand?"
            .to_string();
    }
    criteria
        .iter()
        .map(|c| {
            format!(
                "- {} (weight: {:.0}%): {}",
                c.name,
                c.weight * 100.0,
                c.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse the judge's JSON response into a JudgeOutput.
/// Defensive — every field has a safe fallback so a partial or malformed
/// judge response never crashes the evaluation run.
fn parse_judge_output(json: &Value, had_reference: bool) -> JudgeOutput {
    // dimension_scores arrives as an array of {name, evidence, score, reasoning}
    // (structured outputs can't express a map with arbitrary keys), so we fold
    // it back into the HashMap that JudgeOutput uses, keyed by `name`.
    let dimension_scores: HashMap<String, DimensionScore> = json
        .get("dimension_scores")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let name = item.get("name")?.as_str()?.to_string();
                    Some((
                        name,
                        DimensionScore {
                            score: item.get("score").and_then(|s| s.as_f64()).unwrap_or(5.0),
                            reasoning: item
                                .get("reasoning")
                                .and_then(|r| r.as_str())
                                .unwrap_or("")
                                .to_string(),
                        },
                    ))
                })
                .collect()
        })
        .unwrap_or_default();

    let strengths = json
        .get("strengths")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let weaknesses = json
        .get("weaknesses")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    // If overall_score is absent, compute from dimension means.
    let overall_score = json
        .get("overall_score")
        .and_then(|v| v.as_f64())
        .unwrap_or_else(|| {
            if dimension_scores.is_empty() {
                5.0
            } else {
                dimension_scores.values().map(|d| d.score).sum::<f64>()
                    / dimension_scores.len() as f64
            }
        })
        .max(1.0)
        .min(10.0);

    let judge_reasoning = json
        .get("judge_reasoning")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let reference_used = json
        .get("reference_used")
        .and_then(|v| v.as_bool())
        .unwrap_or(had_reference);

    JudgeOutput {
        dimension_scores,
        strengths,
        weaknesses,
        overall_score,
        judge_reasoning,
        reference_used,
    }
}

// ── Judge ─────────────────────────────────────────────────────────────────────

/// Score one (rendered prompt, response) pair using Claude Sonnet as an independent judge.
///
/// `rendered_prompt` MUST be the exact, verbatim text the model received — template
/// variables already substituted, KB context already injected, question already
/// embedded. The judge is deliberately given nothing else about the input: grading
/// the real prompt (not the raw template or a partial question) is the invariant
/// that keeps it from inventing false "unfilled variable" or "hallucination"
/// findings. This holds for every prompt shape — templated, plain, or KB-augmented —
/// because there is only one thing to grade against: what the model actually saw.
///
/// Accepts an optional rubric (prompt-specific criteria) and optional
/// expected_answer (semantic ground truth). Falls back to generic criteria
/// when neither is provided so the judge is never domain-specific by default.
///
/// The response is constrained to a JSON schema via `send_structured`, so the
/// returned text is guaranteed well-formed JSON. `dimension_scores` is requested
/// as an *array* (structured outputs can't express a map with arbitrary keys);
/// the schema's field order pins reasoning/evidence ahead of every numeric score.
async fn judge_response(
    llm: &AnthropicClient,
    rendered_prompt: &str,
    response: &str,
    expected_answer: Option<&str>,
    rubric: &[RubricCriterion],
) -> Result<JudgeOutput, StatusCode> {
    let criteria_text = build_criteria_text(rubric);

    let reference_section = match expected_answer {
        Some(ea) => format!(
            "\n\nEXPECTED ANSWER SPECIFICATION:\n{}\n\
             (Use this as a semantic reference. The response does not need to \
             match word-for-word, but must satisfy the stated intent.)",
            ea
        ),
        None => String::new(),
    };

    let system = "You are a precise, evidence-based evaluator. \
        Every weakness you report must be supported by a direct quote from the response. \
        Never infer a violation — only flag what is explicitly present in the text. \
        Always respond with valid JSON only.";

    let judge_prompt = format!(
        r#"Assess the AI response below against the evaluation criteria.

PROMPT SENT TO THE MODEL — this is the exact, verbatim text the model received, with every
template variable already substituted and any knowledge-base context already included. Judge
the response against THIS, never against an idealised or templated version:
{}

MODEL RESPONSE:
{}{}

EVALUATION CRITERIA:
{}

PRECISION RULES — read before scoring:
- Judge the response only against the prompt above and the criteria listed. Do not introduce
  criteria that are not present in the rubric.
- GROUNDING (this prevents false "hallucination" and "missing input" findings): every
  instruction, user input, conversation turn, example, value, name, and fact that appears
  anywhere in the PROMPT SENT TO THE MODEL was available to the model. Do NOT flag the model's
  use of anything drawn from the prompt as a fabrication, hallucination, or unfilled/unknown
  variable. The model knows every value present in the prompt. The ONLY genuine "missing
  variable" case is a literal unresolved "{{{{PLACEHOLDER}}}}" marker still visible in the prompt
  text above; if you see none, assume all inputs were provided. Call something a fabrication
  only when it is demonstrably incorrect AND appears nowhere in the prompt.
- Never infer a violation — only flag what is explicitly and unambiguously present in the
  response text.
- Unfamiliar terms: do not flag domain-specific terminology, product names, or specialised
  concepts as errors. The model may have domain knowledge you do not.
- For every weakness you list, you MUST quote the exact phrase from the response that
  constitutes the violation in square brackets, e.g. ["phrase that violates the criterion"].

The "evidence" field in each dimension forces you to reason before scoring — fill it first.
Produce exactly one dimension_scores entry per criterion listed above, naming each one exactly:
{{
  "judge_reasoning": "<2-3 sentences of overall assessment written after reviewing all dimensions>",
  "dimension_scores": [
    {{
      "name": "<criterion name exactly as listed above>",
      "evidence": "<quote from response that most influenced this score, or 'none' if compliant>",
      "score": <1-10>,
      "reasoning": "<one concise sentence>"
    }}
  ],
  "strengths": ["<specific concrete strength with brief quote>"],
  "weaknesses": ["<violation with exact quote in brackets, e.g. used first-person: [I'd recommend]>"],
  "overall_score": <weighted average 1.0-10.0>,
  "reference_used": <true if expected_answer was provided and used, false otherwise>
}}"#,
        rendered_prompt, response, reference_section, criteria_text
    );

    // The API guarantees the response matches this schema, so the returned text
    // is always valid JSON for it. Note the absent numeric bounds on `score`:
    // structured outputs don't support minimum/maximum, so the 1-10 range is
    // enforced in the prompt and clamped (for overall_score) in parse_judge_output.
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "judge_reasoning": { "type": "string" },
            "dimension_scores": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "name": { "type": "string" },
                        "evidence": { "type": "string" },
                        "score": { "type": "number" },
                        "reasoning": { "type": "string" }
                    },
                    "required": ["name", "evidence", "score", "reasoning"]
                }
            },
            "strengths": { "type": "array", "items": { "type": "string" } },
            "weaknesses": { "type": "array", "items": { "type": "string" } },
            "overall_score": { "type": "number" },
            "reference_used": { "type": "boolean" }
        },
        "required": [
            "judge_reasoning", "dimension_scores", "strengths",
            "weaknesses", "overall_score", "reference_used"
        ]
    });

    println!("judge_schema: {:?}", schema);

    let text = llm
        .send_structured(
            llm.model_sonnet(),
            2000,
            &judge_prompt,
            Some(system),
            schema,
        )
        .await?;

    // Parse is still fallible — a refusal or a max_tokens cutoff can return text
    // that doesn't satisfy the schema. Fall back to an all-default JudgeOutput
    // rather than failing the whole evaluation run.
    let json: Value = serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("⚠️  judge_response JSON parse error: {e}\nraw={}", text);
        Value::Null
    });

    Ok(parse_judge_output(&json, expected_answer.is_some()))
}

// ── DB: Dataset resolution ────────────────────────────────────────────────────

async fn fetch_dataset_by_id(
    pool: &PgPool,
    id: &str,
    user_id: &str,
) -> Result<Dataset, StatusCode> {
    sqlx::query_as::<_, Dataset>(
        r#"
        SELECT id, name, question_count, avg_score, evaluations,
               NULL::text as last_used, created_at::text as created_at
        FROM datasets WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("DB error resolving dataset by id: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)
}

/// Try id first, then fall back to dataset name (legacy dataset_path support).
async fn fetch_dataset_by_id_or_name(
    pool: &PgPool,
    key: &str,
    user_id: &str,
) -> Result<Dataset, StatusCode> {
    sqlx::query_as::<_, Dataset>(
        r#"
        SELECT id, name, question_count, avg_score, evaluations,
               NULL::text as last_used, created_at::text as created_at
        FROM datasets WHERE (id = $1 OR name = $1) AND user_id = $2 LIMIT 1
        "#,
    )
    .bind(key)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("DB error resolving dataset by id/name: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)
}

async fn resolve_dataset(
    pool: &PgPool,
    payload: &EvaluationRequest,
    user_id: &str,
) -> Result<Dataset, StatusCode> {
    if let Some(id) = payload.dataset_id.as_ref().filter(|s| !s.is_empty()) {
        return fetch_dataset_by_id(pool, id, user_id).await;
    }
    if let Some(path) = payload.dataset_path.as_ref().filter(|s| !s.is_empty()) {
        return fetch_dataset_by_id_or_name(pool, path, user_id).await;
    }
    Err(StatusCode::BAD_REQUEST)
}

// ── DB: Questions ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct EvalQuestion {
    id: i32,
    question_text: String,
    variable_values: Option<Value>,
    /// Semantic specification of the correct answer — passed to the judge.
    expected_answer: Option<String>,
}

async fn load_questions(pool: &PgPool, dataset_id: &str) -> Result<Vec<EvalQuestion>, StatusCode> {
    let rows = sqlx::query!(
        r#"
        SELECT id, question_text, variable_values, expected_answer
        FROM questions
        WHERE dataset_id = $1
        ORDER BY question_order
        "#,
        dataset_id
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        eprintln!("DB error loading questions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(rows
        .into_iter()
        .map(|r| EvalQuestion {
            id: r.id,
            question_text: r.question_text,
            variable_values: r.variable_values,
            expected_answer: r.expected_answer,
        })
        .collect())
}

// ── DB: Prompt context ────────────────────────────────────────────────────────

/// Everything needed from a prompt row to run an evaluation.
struct PromptContext {
    template: String,
    variables: Vec<String>,
    is_templated: bool,
    rubric: Vec<RubricCriterion>,
    domain: Option<String>,
    expected_output_format: Option<String>,
    use_context: bool,
    context_project: Option<String>,
}

#[derive(sqlx::FromRow)]
struct PromptContextRaw {
    template: String,
    variables: Option<Vec<String>>,
    is_templated: Option<bool>,
    rubric: Option<Value>,
    domain: Option<String>,
    expected_output_format: Option<String>,
    use_context: bool,
    context_project: Option<String>,
}

async fn load_prompt_context(
    pool: &PgPool,
    prompt_id: &str,
    user_id: &str,
) -> Result<PromptContext, StatusCode> {
    let row = sqlx::query_as::<_, PromptContextRaw>(
        r#"
        SELECT template, variables, is_templated, rubric, domain, expected_output_format,
               use_context, context_project
        FROM prompts WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(prompt_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or_else(|| {
        eprintln!("❌ Unknown prompt id (not in DB): {prompt_id}");
        StatusCode::BAD_REQUEST
    })?;

    let variables = row
        .variables
        .unwrap_or_else(|| extract_variables(&row.template));
    let is_templated = row.is_templated.unwrap_or(!variables.is_empty());

    // Deserialise rubric from JSONB → Vec<RubricCriterion>.
    // If NULL or malformed, returns empty vec → build_criteria_text uses generic fallback.
    let rubric: Vec<RubricCriterion> = row
        .rubric
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    Ok(PromptContext {
        template: row.template,
        variables,
        is_templated,
        rubric,
        domain: row.domain,
        expected_output_format: row.expected_output_format,
        use_context: row.use_context,
        context_project: row.context_project,
    })
}

// ── DB: Persist evaluation results ───────────────────────────────────────────

struct EvaluationDetail {
    run_id: String,
    prompt_id: String,
    question_id: i32,
    response: String,
    score: f64,
    strengths: Vec<String>,
    weaknesses: Vec<String>,
    dimension_scores: Value,
    judge_reasoning: String,
    reference_used: bool,
}

/// Insert the run row up front with status='running' so the client gets a
/// durable ID to poll immediately. average_score starts at 0 and per_prompt_scores
/// is left NULL — both are filled in by finalize_evaluation_run when the job ends.
async fn insert_evaluation_run(
    pool: &PgPool,
    run_id: &str,
    dataset_id: &str,
    prompt_ids: &[String],
    total_questions: i32,
    created_at: chrono::DateTime<chrono::Utc>,
    user_id: &str,
) -> Result<(), StatusCode> {
    println!(
        "   [db] insert evaluation_runs id={} prompts={} status=running",
        run_id,
        prompt_ids.len()
    );

    sqlx::query(
        r#"
        INSERT INTO evaluation_runs
            (id, dataset_id, prompt_ids, average_score, total_questions, status, created_at, user_id)
        VALUES ($1, $2, $3, 0, $4, 'running', $5, $6)
        "#,
    )
    .bind(run_id)
    .bind(dataset_id)
    .bind(prompt_ids)
    .bind(total_questions)
    .bind(created_at)
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("Failed to insert evaluation run: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

/// Close out a run: write the final aggregate score, per-prompt breakdown, and
/// terminal status ('completed' or 'failed'). Called once, from the background job.
async fn finalize_evaluation_run(
    pool: &PgPool,
    run_id: &str,
    average_score: f64,
    per_prompt_scores: &HashMap<String, f64>,
    status: &str,
) -> Result<(), StatusCode> {
    let per_prompt_json = serde_json::to_value(per_prompt_scores).unwrap_or_else(|_| json!({}));

    println!(
        "   [db] finalize evaluation_runs id={} status={} avg={:.3}",
        run_id, status, average_score
    );

    sqlx::query(
        r#"
        UPDATE evaluation_runs
        SET average_score = $2, per_prompt_scores = $3, status = $4
        WHERE id = $1
        "#,
    )
    .bind(run_id)
    .bind(average_score)
    .bind(per_prompt_json)
    .bind(status)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("Failed to finalize evaluation run: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

/// Persist one (run, prompt, question) result row. Called from the per-question
/// task the moment a score is ready, so a mid-run failure never discards work
/// already completed — each row is durable as soon as it's scored.
async fn insert_evaluation_detail(
    pool: &PgPool,
    detail: &EvaluationDetail,
) -> Result<(), StatusCode> {
    sqlx::query(
        r#"
        INSERT INTO evaluation_details
            (run_id, question_id, prompt_id, model_answer, score,
             strengths, weaknesses, dimension_scores,
             judge_reasoning, reference_used, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
        "#,
    )
    .bind(&detail.run_id)
    .bind(detail.question_id)
    .bind(&detail.prompt_id)
    .bind(&detail.response)
    .bind(detail.score)
    .bind(detail.strengths.as_slice())
    .bind(detail.weaknesses.as_slice())
    .bind(&detail.dimension_scores)
    .bind(&detail.judge_reasoning)
    .bind(detail.reference_used)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("Failed to save evaluation detail: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

async fn persist_prompt_eval_stats(
    pool: &PgPool,
    prompt_id: &str,
    score_sum: f64,   // sum of all individual question scores for this run
    score_count: i32, // number of questions scored in this run
    user_id: &str,
) -> Result<(), StatusCode> {
    let res = sqlx::query(
        r#"
        UPDATE prompts
        SET
            runs              = runs + 1,
            total_score_sum   = total_score_sum + $1,
            total_score_count = total_score_count + $2,
            average_score     = (total_score_sum + $1) / NULLIF((total_score_count + $2), 0),
            updated_at        = NOW()
        WHERE id = $3 AND user_id = $4
        "#,
    )
    .bind(score_sum)
    .bind(score_count)
    .bind(prompt_id)
    .bind(user_id)
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

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/evaluate
pub async fn run_evaluation(
    State(pool): State<PgPool>,
    State(llm): State<AnthropicClient>,
    State(context_client): State<Option<ContextClient>>,
    user: AuthUser,
    Json(payload): Json<EvaluationRequest>,
) -> Result<Json<EvaluationResult>, StatusCode> {
    println!("📝 Starting evaluation...");
    println!(
        "   dataset_id={:?}  prompt_ids={:?}",
        payload.dataset_id, payload.prompt_ids
    );

    // Validate inputs synchronously so the client gets a 4xx now, before we spawn.
    // resolve_dataset is user-scoped, so a foreign/unknown dataset is a 404 here.
    let dataset = resolve_dataset(&pool, &payload, &user.user_id).await?;
    let questions = load_questions(&pool, &dataset.id).await?;

    if questions.is_empty() {
        eprintln!("❌ Dataset {} has no questions", dataset.id);
        return Err(StatusCode::BAD_REQUEST);
    }

    println!(
        "   dataset=\"{}\" questions={}",
        dataset.name,
        questions.len()
    );

    let run_id = format!("eval_{}", chrono::Utc::now().timestamp());
    let created_at = chrono::Utc::now();
    let total_questions = questions.len() as i32;

    // Persist the run as 'running' before returning, so the ID the client polls is
    // already backed by a row. If this insert fails there's nothing to clean up yet.
    insert_evaluation_run(
        &pool,
        &run_id,
        &dataset.id,
        &payload.prompt_ids,
        total_questions,
        created_at,
        &user.user_id,
    )
    .await?;

    // Hand the heavy work (potentially hundreds of LLM calls) to a background task.
    // tokio::spawn requires a 'static future, so we move owned clones in — PgPool and
    // AnthropicClient are cheap to clone (both wrap an Arc internally). The owner id
    // is moved in too so the job's prompt loads/stats stay scoped to this user.
    tokio::spawn(run_evaluation_job(
        pool.clone(),
        llm.clone(),
        context_client.clone(),
        run_id.clone(),
        payload.prompt_ids.clone(),
        questions,
        user.user_id.clone(),
    ));

    // Return immediately. The frontend polls GET /api/evaluations/:id until `status`
    // leaves 'running'.
    Ok(Json(EvaluationResult {
        id: run_id,
        status: "running".to_string(),
        average_score: 0.0,
        total_items: total_questions,
        scores: vec![],
        dataset: dataset.name,
        prompts: payload.prompt_ids,
        per_prompt_scores: HashMap::new(),
        created_at,
    }))
}

/// Max questions evaluated concurrently within a single prompt. Bounded by the DB
/// pool (`max_connections(5)` in db.rs): each task briefly grabs a connection to
/// persist its detail row, and we leave headroom for the poll/list endpoints that
/// hit the same pool while a run is in flight. Raise this and the pool together.
const QUESTION_CONCURRENCY: usize = 3;

/// The background half of an evaluation. Runs every prompt × question pair,
/// persisting each result the moment it's scored, then writes the terminal status.
/// Takes everything by value because it outlives the request that spawned it.
async fn run_evaluation_job(
    pool: PgPool,
    llm: AnthropicClient,
    context_client: Option<ContextClient>,
    run_id: String,
    prompt_ids: Vec<String>,
    questions: Vec<EvalQuestion>,
    user_id: String,
) {
    let mut all_scores: Vec<f64> = vec![];
    let mut per_prompt_scores: HashMap<String, f64> = HashMap::new();

    for prompt_id in &prompt_ids {
        println!("\n🔄 Testing prompt_id={}", prompt_id);

        // user-scoped: a prompt that isn't this user's is skipped (treated as missing).
        let prompt = match load_prompt_context(&pool, prompt_id, &user_id).await {
            Ok(p) => p,
            Err(e) => {
                eprintln!("❌ Skipping prompt {prompt_id}: failed to load context: {e:?}");
                continue;
            }
        };
        println!(
            "   is_templated={}  domain={:?}  rubric_criteria={}",
            prompt.is_templated,
            prompt.domain,
            prompt.rubric.len()
        );

        // Run this prompt's questions concurrently. The futures borrow pool/llm/
        // prompt/run_id; buffer_unordered keeps at most QUESTION_CONCURRENCY in
        // flight, and the stream is fully awaited before `prompt` drops.
        //
        // We stream over indices rather than `questions.iter()` deliberately: a
        // closure taking `&EvalQuestion` would need to be lifetime-general for any
        // borrow, which `tokio::spawn`'s 'static bound rejects ("FnOnce is not
        // general enough"). Taking an owned `usize` and indexing inside sidesteps it.
        let scored: Vec<Option<f64>> = stream::iter(0..questions.len())
            .map(|i| {
                evaluate_one_question(
                    &pool,
                    &llm,
                    &context_client,
                    &run_id,
                    prompt_id,
                    &prompt,
                    &questions[i],
                    i,
                )
            })
            .buffer_unordered(QUESTION_CONCURRENCY)
            .collect()
            .await;

        let run_scores: Vec<f64> = scored.into_iter().flatten().collect();
        let score_sum: f64 = run_scores.iter().sum();
        let score_count = run_scores.len() as i32;

        if score_count > 0 {
            let run_avg = score_sum / score_count as f64;
            println!(
                "   prompt_id={}  run_avg={:.3}  (sum={:.1} / count={})",
                prompt_id, run_avg, score_sum, score_count
            );
            per_prompt_scores.insert(prompt_id.clone(), run_avg);

            // Best-effort: a stats update failure shouldn't sink the run's own results.
            if let Err(e) =
                persist_prompt_eval_stats(&pool, prompt_id, score_sum, score_count, &user_id).await
            {
                eprintln!("⚠️  prompt stats update failed for {prompt_id}: {e:?}");
            }
            all_scores.extend(run_scores);
        } else {
            eprintln!("⚠️  prompt {prompt_id} produced no scored questions");
        }
    }

    // No successful results at all → mark failed so the client stops polling and
    // surfaces an error, rather than reporting a meaningless 0.0 average.
    let (status, overall_avg) = if all_scores.is_empty() {
        ("failed", 0.0)
    } else {
        (
            "completed",
            all_scores.iter().sum::<f64>() / all_scores.len() as f64,
        )
    };

    println!(
        "\n📦 run_id={}  status={}  total_scored={}  overall_avg={:.3}",
        run_id,
        status,
        all_scores.len(),
        overall_avg
    );

    if let Err(e) =
        finalize_evaluation_run(&pool, &run_id, overall_avg, &per_prompt_scores, status).await
    {
        eprintln!("❌ Failed to finalize run {run_id}: {e:?}");
    }
}

/// Evaluate a single (prompt, question): build the prompt, optionally inject KB
/// context, generate with Haiku, judge with Sonnet, and persist the detail row.
/// Returns the score on success, or `None` on any failure (logged) — one bad
/// question is skipped, never allowed to sink the whole run.
async fn evaluate_one_question(
    pool: &PgPool,
    llm: &AnthropicClient,
    context_client: &Option<ContextClient>,
    run_id: &str,
    prompt_id: &str,
    prompt: &PromptContext,
    question: &EvalQuestion,
    index: usize,
) -> Option<f64> {
    println!(
        "   Q{}  id={}  text=\"{}\"",
        index + 1,
        question.id,
        preview(&question.question_text, 120)
    );

    // Fill template variables or fall back to appending the question directly.
    let mut full_prompt = if prompt.is_templated {
        if let Some(vars) = &question.variable_values {
            fill_template(&prompt.template, vars)
        } else {
            fill_template(
                &prompt.template,
                &json!({ "QUESTION": question.question_text }),
            )
        }
    } else {
        format!(
            "{}\n\nUser question: {}",
            prompt.template, question.question_text
        )
    };

    // Fetch knowledge base context if this prompt is configured to use it and
    // inject it into full_prompt. We don't track it separately any more — the judge
    // sees it as part of the rendered prompt, which is the single source of truth.
    if prompt.use_context {
        match (context_client, &prompt.context_project) {
            (Some(client), Some(project)) => {
                match client.fetch_context(&question.question_text, project).await {
                    Ok(ctx) if !ctx.is_empty() => {
                        full_prompt.push_str(
                            "\n\n## Knowledge Base Context\n\
                            The following information is relevant to this question. \
                            Draw on these specifics in your response.\n\n",
                        );
                        full_prompt.push_str(&ctx);
                        full_prompt.push_str(
                            "\n\n---\n\
                            Now respond to the question above, following all instructions \
                            and drawing on the knowledge provided where relevant.",
                        );
                        println!("🔍 context injected ({} chars)", ctx.len());
                    }
                    Ok(_) => println!("⚠️  context API returned empty result"),
                    Err(e) => eprintln!("⚠️  context API error (continuing without): {e}"),
                }
            }
            (None, _) => eprintln!("⚠️  use_context=true but CONTEXT_ENGINE_URL/KEY not set"),
            (_, None) => eprintln!("⚠️  use_context=true but context_project not set on prompt"),
        }
    }

    // For non-templated prompts the question has not yet been appended — add it now.
    // Templated prompts already embed the question via {{USER_MESSAGE}} substitution,
    // so we skip this to avoid duplicating the question with the wrong framing.
    if !prompt.is_templated {
        full_prompt.push_str("\n\n---\n\nUser question: ");
        full_prompt.push_str(&question.question_text);
        full_prompt.push_str("\n\nYour response:");
    }

    // Generate model response (Haiku — cost-efficient for bulk generation).
    let response = match llm.send_text(llm.model_haiku(), 1000, &full_prompt, None).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Haiku call failed for question {}: {:?}", question.id, e);
            return None;
        }
    };

    println!(
        "   ✅ response chars={}  preview=\"{}\"",
        response.chars().count(),
        preview(&response, 120)
    );

    // Judge the response (Sonnet — stronger model as independent evaluator).
    // Pass the exact rendered prompt the model received — never the raw template —
    // so the judge grades reality (see judge_response's contract).
    let judge = match judge_response(
        llm,
        &full_prompt,
        &response,
        question.expected_answer.as_deref(),
        &prompt.rubric,
    )
    .await
    {
        Ok(j) => j,
        Err(e) => {
            eprintln!("❌ Judge failed for question {}: {:?}", question.id, e);
            return None;
        }
    };

    println!(
        "   📊 score={:.1}  reference_used={}  strengths={}  weaknesses={}",
        judge.overall_score,
        judge.reference_used,
        judge.strengths.len(),
        judge.weaknesses.len()
    );

    let detail = EvaluationDetail {
        run_id: run_id.to_string(),
        prompt_id: prompt_id.to_string(),
        question_id: question.id,
        response,
        score: judge.overall_score,
        strengths: judge.strengths,
        weaknesses: judge.weaknesses,
        dimension_scores: serde_json::to_value(&judge.dimension_scores).unwrap_or(Value::Null),
        judge_reasoning: judge.judge_reasoning,
        reference_used: judge.reference_used,
    };

    // Persist before returning the score. If the row doesn't land, treat the
    // question as failed so the run's average stays consistent with what
    // GET /api/evaluations/:id recomputes from the detail rows.
    if let Err(e) = insert_evaluation_detail(pool, &detail).await {
        eprintln!(
            "❌ Failed to persist detail for question {}: {:?}",
            question.id, e
        );
        return None;
    }

    Some(detail.score)
}

#[derive(sqlx::FromRow)]
struct EvalRunListRow {
    id: String,
    status: Option<String>,
    prompt_ids: Vec<String>,
    average_score: f64,
    total_questions: i32,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
    per_prompt_scores: Option<Value>,
    dataset_name: Option<String>,
    scores: Option<Vec<f64>>,
}

#[derive(sqlx::FromRow)]
struct EvalRunRow {
    id: String,
    status: Option<String>,
    dataset_id: Option<String>,
    prompt_ids: Vec<String>,
    average_score: f64,
    total_questions: i32,
    per_prompt_scores: Option<Value>,
    created_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(sqlx::FromRow)]
struct EvalDetailRow {
    prompt_id: Option<String>,
    question_text: String,
    model_answer: Option<String>,
    score: Option<f64>,
    strengths: Option<Vec<String>>,
    weaknesses: Option<Vec<String>>,
    dimension_scores: Option<Value>,
    judge_reasoning: Option<String>,
    reference_used: Option<bool>,
}

/// GET /api/evaluations
pub async fn list_evaluations(
    State(pool): State<PgPool>,
    user: AuthUser,
) -> Result<Json<Vec<EvaluationResult>>, StatusCode> {
    println!("📋 Listing evaluations for user {}", user.user_id);

    let rows = sqlx::query_as::<_, EvalRunListRow>(
        r#"
        SELECT
            er.id,
            er.status,
            er.prompt_ids,
            er.average_score,
            er.total_questions,
            er.created_at,
            er.per_prompt_scores,
            d.name as dataset_name,
            array_agg(ed.score ORDER BY ed.id) as scores
        FROM evaluation_runs er
        LEFT JOIN datasets d ON er.dataset_id = d.id
        LEFT JOIN evaluation_details ed ON er.id = ed.run_id
        WHERE er.user_id = $1
        GROUP BY er.id, er.status, er.prompt_ids, er.average_score, er.total_questions,
                 er.created_at, er.per_prompt_scores, d.name
        ORDER BY er.created_at DESC
        LIMIT 50
        "#,
    )
    .bind(&user.user_id)
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("DB error listing evaluations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let results = rows
        .into_iter()
        .map(|row| {
            let per_prompt_scores: HashMap<String, f64> = row
                .per_prompt_scores
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default();

            EvaluationResult {
                id: row.id,
                status: row.status.unwrap_or_else(|| "completed".to_string()),
                average_score: row.average_score,
                total_items: row.total_questions,
                scores: row.scores.unwrap_or_default(),
                dataset: row.dataset_name.unwrap_or_default(),
                prompts: row.prompt_ids,
                per_prompt_scores,
                created_at: row.created_at.unwrap(),
            }
        })
        .collect();

    Ok(Json(results))
}

/// GET /api/evaluations/:id
pub async fn get_evaluation(
    State(pool): State<PgPool>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<EvaluationWithDetails>, StatusCode> {
    println!("🔍 Getting evaluation: {}", id);

    let run = sqlx::query_as::<_, EvalRunRow>(
        r#"
        SELECT id, status, dataset_id, prompt_ids, average_score, total_questions,
               per_prompt_scores, created_at
        FROM evaluation_runs
        WHERE id = $1 AND user_id = $2
        "#,
    )
    .bind(&id)
    .bind(&user.user_id)
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

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

    let details_rows = sqlx::query_as::<_, EvalDetailRow>(
        r#"
        SELECT
            ed.prompt_id,
            q.question_text,
            ed.model_answer,
            ed.score,
            ed.strengths,
            ed.weaknesses,
            ed.dimension_scores,
            ed.judge_reasoning,
            ed.reference_used
        FROM evaluation_details ed
        JOIN questions q ON ed.question_id = q.id
        WHERE ed.run_id = $1
        ORDER BY ed.id
        "#,
    )
    .bind(&id)
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let scores: Vec<f64> = details_rows.iter().filter_map(|d| d.score).collect();
    let created_at = run.created_at.unwrap_or_else(chrono::Utc::now);

    let per_prompt_scores: HashMap<String, f64> = run
        .per_prompt_scores
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let details = details_rows
        .into_iter()
        .map(|d| QuestionDetail {
            prompt_id: d.prompt_id.unwrap_or_default(),
            question: d.question_text,
            response: d.model_answer.unwrap_or_default(),
            score: d.score.unwrap_or(0.0),
            strengths: d.strengths,
            weaknesses: d.weaknesses,
            dimension_scores: d.dimension_scores,
            judge_reasoning: d.judge_reasoning,
            reference_used: d.reference_used.unwrap_or(false),
        })
        .collect();

    Ok(Json(EvaluationWithDetails {
        id: run.id,
        status: run.status.unwrap_or_else(|| "completed".to_string()),
        average_score: run.average_score,
        total_items: run.total_questions,
        scores,
        dataset: dataset_name,
        prompts: run.prompt_ids,
        per_prompt_scores,
        created_at,
        details,
    }))
}

// ── Test case generation ──────────────────────────────────────────────────────

/// POST /api/questions/generate
pub async fn generate_test_cases(
    State(pool): State<PgPool>,
    State(llm): State<AnthropicClient>,
    State(context_client): State<Option<ContextClient>>,
    user: AuthUser,
    Json(payload): Json<GenerateTestCasesRequest>,
) -> Result<Json<GenerateTestCasesResponse>, StatusCode> {
    let count = payload.count.clamp(1, 20);
    // user-scoped: generating cases for a prompt you don't own is a 4xx (NOT_FOUND).
    let prompt = load_prompt_context(&pool, &payload.prompt_id, &user.user_id).await?;

    // Fetch KB context if the prompt is configured to use it.
    // Seed query is built from domain + rubric criteria so the generator
    // receives relevant facts without needing a specific question.
    let kb_context: Option<String> = if prompt.use_context {
        match (&context_client, &prompt.context_project) {
            (Some(client), Some(project)) => {
                let domain = prompt.domain.as_deref().unwrap_or("general");
                let criteria = prompt
                    .rubric
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                let seed = format!("{domain} {criteria} services capabilities expertise");
                println!("🔍 Fetching KB context for test case generation (seed: \"{seed}\")");
                match client.fetch_context(&seed, project).await {
                    Ok(ctx) if !ctx.is_empty() => {
                        println!(
                            "🔍 KB context fetched ({} chars) — injecting into generator",
                            ctx.len()
                        );
                        Some(ctx)
                    }
                    Ok(_) => {
                        println!("⚠️  KB context empty for seed query");
                        None
                    }
                    Err(e) => {
                        eprintln!("⚠️  KB context fetch failed (continuing without): {e}");
                        None
                    }
                }
            }
            (None, _) => {
                eprintln!("⚠️  use_context=true but context engine not configured");
                None
            }
            (_, None) => {
                eprintln!("⚠️  use_context=true but context_project not set");
                None
            }
        }
    } else {
        None
    };

    let cases = generate_test_cases_with_ai(&llm, &prompt, count, kb_context.as_deref()).await?;
    Ok(Json(GenerateTestCasesResponse { test_cases: cases }))
}

async fn generate_test_cases_with_ai(
    llm: &AnthropicClient,
    prompt: &PromptContext,
    count: i32,
    kb_context: Option<&str>,
) -> Result<Vec<GeneratedTestCase>, StatusCode> {
    // Build rubric text for the generator so it can produce cases that probe specific criteria.
    let rubric_text = if prompt.rubric.is_empty() {
        "Generic: relevance, accuracy, completeness, clarity".to_string()
    } else {
        prompt
            .rubric
            .iter()
            .map(|c| {
                format!(
                    "- {} (weight {:.0}%): {}",
                    c.name,
                    c.weight * 100.0,
                    c.description
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let domain = prompt.domain.as_deref().unwrap_or("general");
    let output_format = prompt
        .expected_output_format
        .as_deref()
        .unwrap_or("clear, helpful response");

    /*
     * This is the knowledge base context that is injected into the prompt.
     * It is used to generate test cases that are grounded in reality.
     */

    let kb_section = match kb_context {
        Some(ctx) => format!(
            "\n\nKNOWLEDGE BASE CONTEXT:\n{}\n\n\
             GROUNDING RULE: When generating variable_values and expected_answer, \
             prefer referencing real entities, services, and terminology from the \
             knowledge base above over invented examples. Invent only when the \
             knowledge base lacks coverage for a needed scenario.",
            ctx
        ),
        None => String::new(),
    };

    // For non-templated prompts (no variables), the question text has nowhere to
    // live except variable_values. We instruct Sonnet to use the key "QUESTION"
    // so the frontend's preferred-key extraction can find it.
    let variable_instruction = if prompt.variables.is_empty() {
        "This prompt has NO template variables — it is a fixed system prompt. \
         For each test case, put the user's question text into variable_values \
         as {\"QUESTION\": \"<the question the user would ask>\"}. \
         The QUESTION value is what will be sent to the model as the user turn."
            .to_string()
    } else {
        format!(
            "TEMPLATE VARIABLES: {:?}\n\
             Set realistic values for every variable listed above.",
            prompt.variables
        )
    };

    let user_prompt = format!(
        r#"You are a test-case engineer generating evaluation data for an LLM prompt.

DOMAIN: {}
EXPECTED OUTPUT FORMAT: {}

PROMPT TEMPLATE:
{}

{}

EVALUATION RUBRIC:
{}{}

Generate EXACTLY {} test cases that comprehensively exercise this prompt. The
"test_cases" array MUST contain exactly that many items — do not return fewer.

DIFFICULTY DISTRIBUTION:
- ~30% easy   (straightforward happy-path)
- ~40% medium (typical use with some complexity)
- ~20% hard   (ambiguous, multi-layered, edge cases)
- ~10% adversarial (probes a specific rubric failure mode)
If count < 3, use medium cases. Always include at least 1 hard if count >= 3.

Each case must:
1. Set variable_values as instructed above
2. Write expected_answer as a semantic SPECIFICATION (not verbatim) of what a
   correct response must contain — e.g. "Must ask exactly one question in English
   without revealing the answer and acknowledge the student's frustration."
3. Choose a meaningful case_type (happy_path, edge_case, emotional_stress,
   out_of_scope, boundary_value, adversarial, multi_part, etc.)
4. Choose tags that identify the rubric dimension being probed and the difficulty

Return the cases under a "test_cases" array."#,
        domain,
        output_format,
        prompt.template,
        variable_instruction,
        rubric_text,
        kb_section,
        count,
    );

    // Pin variable_values to explicit string properties. Structured outputs can't
    // express an object with arbitrary keys (additionalProperties:false + no
    // properties = empty object only), but we know the exact variable names here —
    // enumerating them constrains the model to fill every one and nothing else.
    let var_names: Vec<&str> = if prompt.variables.is_empty() {
        vec!["QUESTION"]
    } else {
        prompt.variables.iter().map(String::as_str).collect()
    };
    let var_props = Value::Object(
        var_names
            .iter()
            .map(|name| (name.to_string(), json!({ "type": "string" })))
            .collect(),
    );
    let var_required = Value::Array(var_names.iter().map(|n| Value::String(n.to_string())).collect());

    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "test_cases": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        "variable_values": {
                            "type": "object",
                            "additionalProperties": false,
                            "properties": var_props,
                            "required": var_required
                        },
                        "expected_answer": { "type": "string" },
                        "difficulty": {
                            "type": "string",
                            "enum": ["easy", "medium", "hard", "adversarial"]
                        },
                        "case_type": { "type": "string" },
                        "tags": { "type": "array", "items": { "type": "string" } },
                        "reasoning": { "type": "string" }
                    },
                    "required": [
                        "variable_values", "expected_answer", "difficulty",
                        "case_type", "tags", "reasoning"
                    ]
                }
            }
        },
        "required": ["test_cases"]
    });

    // Schema-constrained output → guaranteed valid JSON in the test_cases shape, so
    // the old two-pass retry + fence-stripping + substring extraction is no longer
    // needed. Note: structured outputs can't enforce array length (no minItems),
    // so the prompt asks for `count` and the .take(count) below still caps it.
    let text = llm
        .send_structured(llm.model_sonnet(), 4000, &user_prompt, None, schema)
        .await?;

    let parsed: Value = serde_json::from_str(&text).map_err(|e| {
        eprintln!("generate_test_cases JSON parse error: {e}\nraw={text}");
        StatusCode::BAD_GATEWAY
    })?;
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
            expected_answer: v
                .get("expected_answer")
                .and_then(|e| e.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from),
            difficulty: v
                .get("difficulty")
                .and_then(|d| d.as_str())
                .unwrap_or("medium")
                .to_string(),
            case_type: v
                .get("case_type")
                .and_then(|t| t.as_str())
                .unwrap_or("happy_path")
                .to_string(),
            tags: v
                .get("tags")
                .and_then(|t| t.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default(),
            reasoning: v
                .get("reasoning")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string(),
        })
        .collect();

    Ok(test_cases)
}

// Unit tests for the generate_test_cases_with_ai function

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_criteria_empty_rubric() {
        let criteria: Vec<RubricCriterion> = vec![];

        let text = build_criteria_text(&criteria);

        assert!(text.contains("relevance"));
        assert!(text.contains("accuracy"));
        assert!(text.contains("completeness"));
        assert!(text.contains("clarity"));
    }

    #[test]
    fn build_criteria_custom_rubric() {
        let criteria: Vec<RubricCriterion> = vec![
            RubricCriterion {
                name: "tone".to_string(),
                description: "Is the response tone appropriate for the task?".to_string(),
                weight: 0.25,
            },
            RubricCriterion {
                name: "safety".to_string(),
                description: "Is the response safe to use in a production environment?".to_string(),
                weight: 0.25,
            },
            RubricCriterion {
                name: "usefulness".to_string(),
                description: "Is the response useful for the task?".to_string(),
                weight: 0.25,
            },
            RubricCriterion {
                name: "consistency".to_string(),
                description: "Is the response consistent in its tone and style?".to_string(),
                weight: 0.25,
            },
        ];

        let text = build_criteria_text(&criteria);

        let expected = ["tone", "safety", "usefulness", "consistency"];

        for item in expected {
            assert!(text.contains(item), "missing criterion: {}", item);
            assert!(text.contains("25%"), "missing weight percentage");
        }
    }

    #[test]
    fn parse_judge_output_valid() {
        let json = json!({
            "dimension_scores": {
                "tone": {
                    "score": 8.0,
                    "reasoning": "The response has a professional and friendly tone."
                }
            }
        });
        let output = parse_judge_output(&json, false);
        assert_eq!(output.dimension_scores.len(), 1);
        assert_eq!(output.dimension_scores["tone"].score, 8.0);
        assert_eq!(
            output.dimension_scores["tone"].reasoning,
            "The response has a professional and friendly tone."
        );
    }
}
