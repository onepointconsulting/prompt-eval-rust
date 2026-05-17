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
    let dimension_scores: HashMap<String, DimensionScore> = json
        .get("dimension_scores")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        DimensionScore {
                            score: v.get("score").and_then(|s| s.as_f64()).unwrap_or(5.0),
                            reasoning: v
                                .get("reasoning")
                                .and_then(|r| r.as_str())
                                .unwrap_or("")
                                .to_string(),
                        },
                    )
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

/// Score one (question, response) pair using Claude Sonnet as an independent judge.
///
/// Accepts an optional rubric (prompt-specific criteria) and optional
/// expected_answer (semantic ground truth). Falls back to generic criteria
/// when neither is provided so the judge is never domain-specific by default.
///
/// The prompt puts judge_reasoning first — this is the G-Eval pattern: force
/// the model to articulate reasoning before committing to numeric scores,
/// which produces more consistent and calibrated outputs.
async fn judge_response(
    llm: &AnthropicClient,
    question: &str,
    response: &str,
    expected_answer: Option<&str>,
    rubric: &[RubricCriterion],
    prompt_template: Option<&str>,
    kb_context: Option<&str>,
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

    let prompt_context = match prompt_template {
        Some(t) => format!(
            "\n\nSYSTEM PROMPT UNDER EVALUATION:\n{}\n\
             (The response was generated by an LLM using this system prompt. \
             Judge whether the response follows the rules and instructions \
             defined here. The rubric criteria below indicate which dimensions \
             to score and their relative weights.)",
            t
        ),
        None => String::new(),
    };

    let kb_section = match kb_context {
        Some(ctx) => format!(
            "\n\nKNOWLEDGE BASE CONTEXT (available to the model during generation):\n{}\n\
             (The model had access to this information when generating the response. \
             Any service names, product names, facts, or claims that appear in this \
             context are KB-grounded and must NOT be flagged as fabrications.)",
            ctx
        ),
        None => String::new(),
    };

    let system = "You are a precise, evidence-based evaluator. \
        Every weakness you report must be supported by a direct quote from the response. \
        Never infer a violation — only flag what is explicitly present in the text. \
        Always respond with valid JSON only.";

    let judge_prompt = format!(
        r#"Assess the quality of the AI response below against the evaluation criteria.

ORIGINAL QUESTION:
{}

AI RESPONSE:
{}{}{}{}

EVALUATION CRITERIA:
{}

PRECISION RULES — read before scoring:
- Only flag a violation if it is directly supported by the evaluation criteria listed above.
  Do not introduce criteria that are not present in the rubric.
- Never infer a violation — only flag what is explicitly and unambiguously present in the
  response text.
- Unfamiliar terms: do not flag domain-specific terminology, product names, or specialised
  concepts as errors. The model may have domain knowledge you do not. Only flag fabrication
  if a claim is demonstrably incorrect.
- For every weakness you list, you MUST quote the exact phrase from the response that
  constitutes the violation in square brackets, e.g. ["phrase that violates the criterion"].

Respond with ONLY valid JSON — no markdown fences, no prose before or after.
The "evidence" field in each dimension forces you to reason before scoring — fill it first:
{{
  "pre_reasoning": "<quote specific phrases from the response for each criterion before scoring>",
  "judge_reasoning": "<2-3 sentences of overall assessment written after reviewing all dimensions>",
  "dimension_scores": {{
    "<criterion_name>": {{
      "evidence": "<quote from response that most influenced this score, or 'none' if compliant>",
      "score": <1-10>,
      "reasoning": "<one concise sentence>"
    }}
  }},
  "strengths": ["<specific concrete strength with brief quote>"],
  "weaknesses": ["<violation with exact quote in brackets, e.g. used first-person: [I'd recommend]>"],
  "overall_score": <weighted average 1.0-10.0>,
  "reference_used": <true if expected_answer was provided and used, false otherwise>
}}"#,
        question, response, reference_section, prompt_context, kb_section, criteria_text
    );

    let text = llm
        .send_text(llm.model_sonnet(), 2000, &judge_prompt, Some(system))
        .await?;

    // Strip markdown fences in case the model ignores the instruction.
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json: Value = serde_json::from_str(cleaned).unwrap_or_else(|e| {
        eprintln!("⚠️  judge_response JSON parse error: {e}\nraw={}", cleaned);
        Value::Null
    });

    Ok(parse_judge_output(&json, expected_answer.is_some()))
}

// ── DB: Dataset resolution ────────────────────────────────────────────────────

async fn fetch_dataset_by_id(pool: &PgPool, id: &str) -> Result<Dataset, StatusCode> {
    sqlx::query_as::<_, Dataset>(
        r#"
        SELECT id, name, question_count, avg_score, evaluations,
               NULL::text as last_used, created_at::text as created_at
        FROM datasets WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        eprintln!("DB error resolving dataset by id: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)
}

/// Try id first, then fall back to dataset name (legacy dataset_path support).
async fn fetch_dataset_by_id_or_name(pool: &PgPool, key: &str) -> Result<Dataset, StatusCode> {
    sqlx::query_as::<_, Dataset>(
        r#"
        SELECT id, name, question_count, avg_score, evaluations,
               NULL::text as last_used, created_at::text as created_at
        FROM datasets WHERE id = $1 OR name = $1 LIMIT 1
        "#,
    )
    .bind(key)
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
) -> Result<Dataset, StatusCode> {
    if let Some(id) = payload.dataset_id.as_ref().filter(|s| !s.is_empty()) {
        return fetch_dataset_by_id(pool, id).await;
    }
    if let Some(path) = payload.dataset_path.as_ref().filter(|s| !s.is_empty()) {
        return fetch_dataset_by_id_or_name(pool, path).await;
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

async fn load_prompt_context(pool: &PgPool, prompt_id: &str) -> Result<PromptContext, StatusCode> {
    let row = sqlx::query_as::<_, PromptContextRaw>(
        r#"
        SELECT template, variables, is_templated, rubric, domain, expected_output_format,
               use_context, context_project
        FROM prompts WHERE id = $1
        "#,
    )
    .bind(prompt_id)
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

async fn save_evaluation_run(
    pool: &PgPool,
    run_id: &str,
    dataset_id: &str,
    prompt_ids: &[String],
    average_score: f64,
    total_questions: i32,
    per_prompt_scores: &HashMap<String, f64>,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), StatusCode> {
    let per_prompt_json = serde_json::to_value(per_prompt_scores).unwrap_or_else(|_| json!({}));

    println!(
        "   [db] insert evaluation_runs id={} prompts={} avg={:.3} per_prompt={:?}",
        run_id,
        prompt_ids.len(),
        average_score,
        per_prompt_scores
    );

    sqlx::query(
        r#"
        INSERT INTO evaluation_runs
            (id, dataset_id, prompt_ids, average_score, total_questions,
             status, per_prompt_scores, created_at)
        VALUES ($1, $2, $3, $4, $5, 'completed', $6, $7)
        "#,
    )
    .bind(run_id)
    .bind(dataset_id)
    .bind(prompt_ids)
    .bind(average_score)
    .bind(total_questions)
    .bind(per_prompt_json)
    .bind(created_at)
    .execute(pool)
    .await
    .map_err(|e| {
        eprintln!("Failed to save evaluation run: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(())
}

async fn save_evaluation_details(
    pool: &PgPool,
    details: &[EvaluationDetail],
) -> Result<(), StatusCode> {
    println!(
        "   [db] inserting {} evaluation_details rows",
        details.len()
    );

    for detail in details {
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
    }

    Ok(())
}

async fn persist_prompt_eval_stats(
    pool: &PgPool,
    prompt_id: &str,
    score_sum: f64,   // sum of all individual question scores for this run
    score_count: i32, // number of questions scored in this run
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
        WHERE id = $3
        "#,
    )
    .bind(score_sum)
    .bind(score_count)
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

// ── Handlers ──────────────────────────────────────────────────────────────────

/// POST /api/evaluate
pub async fn run_evaluation(
    State(pool): State<PgPool>,
    State(llm): State<AnthropicClient>,
    State(context_client): State<Option<ContextClient>>,
    Json(payload): Json<EvaluationRequest>,
) -> Result<Json<EvaluationResult>, StatusCode> {
    println!("📝 Starting evaluation...");
    println!(
        "   dataset_id={:?}  prompt_ids={:?}",
        payload.dataset_id, payload.prompt_ids
    );

    let dataset = resolve_dataset(&pool, &payload).await?;
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

    let mut all_scores: Vec<f64> = vec![];
    let mut all_details: Vec<EvaluationDetail> = vec![];
    let mut per_prompt_scores: HashMap<String, f64> = HashMap::new();

    for prompt_id in &payload.prompt_ids {
        println!("\n🔄 Testing prompt_id={}", prompt_id);

        let prompt = load_prompt_context(&pool, prompt_id).await?;
        println!(
            "   is_templated={}  domain={:?}  rubric_criteria={}",
            prompt.is_templated,
            prompt.domain,
            prompt.rubric.len()
        );

        let mut run_scores: Vec<f64> = vec![];

        for (i, question) in questions.iter().enumerate() {
            println!(
                "   Q{}  id={}  text=\"{}\"",
                i + 1,
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

            // Fetch knowledge base context if this prompt is configured to use it.
            // Also capture the raw context string so we can pass it to the judge —
            // the judge needs to know what information was available to verify claims.
            let mut fetched_kb_context: Option<String> = None;
            if prompt.use_context {
                match (&context_client, &prompt.context_project) {
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
                                fetched_kb_context = Some(ctx);
                            }
                            Ok(_) => println!("⚠️  context API returned empty result"),
                            Err(e) => {
                                eprintln!("⚠️  context API error (continuing without): {e}")
                            }
                        }
                    }
                    (None, _) => {
                        eprintln!("⚠️  use_context=true but CONTEXT_ENGINE_URL/KEY not set")
                    }
                    (_, None) => {
                        eprintln!("⚠️  use_context=true but context_project not set on prompt")
                    }
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

            // Log the tail of the full prompt so we can confirm context is being seen by Haiku.
            let prompt_chars: Vec<char> = full_prompt.chars().collect();
            let tail_start = prompt_chars.len().saturating_sub(300);
            let tail: String = prompt_chars[tail_start..].iter().collect();
            println!("📝 prompt tail (last 300 chars): {:?}", tail);

            // Generate model response (Haiku — cost-efficient for bulk generation).
            let response = llm
                .send_text(llm.model_haiku(), 1000, &full_prompt, None)
                .await
                .map_err(|e| {
                    eprintln!("❌ Haiku call failed for question {}: {:?}", question.id, e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;

            println!(
                "   ✅ response chars={}  preview=\"{}\"",
                response.chars().count(),
                preview(&response, 120)
            );

            // Judge the response (Sonnet — stronger model as independent evaluator).
            let judge = judge_response(
                &llm,
                &question.question_text,
                &response,
                question.expected_answer.as_deref(),
                &prompt.rubric,
                Some(&prompt.template),
                fetched_kb_context.as_deref(),
            )
            .await
            .map_err(|e| {
                eprintln!("❌ Judge failed for question {}: {:?}", question.id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            println!(
                "   📊 score={:.1}  reference_used={}  strengths={}  weaknesses={}",
                judge.overall_score,
                judge.reference_used,
                judge.strengths.len(),
                judge.weaknesses.len()
            );

            run_scores.push(judge.overall_score);
            all_scores.push(judge.overall_score);

            all_details.push(EvaluationDetail {
                run_id: run_id.clone(),
                prompt_id: prompt_id.clone(),
                question_id: question.id,
                response,
                score: judge.overall_score,
                strengths: judge.strengths,
                weaknesses: judge.weaknesses,
                dimension_scores: serde_json::to_value(&judge.dimension_scores)
                    .unwrap_or(Value::Null),
                judge_reasoning: judge.judge_reasoning,
                reference_used: judge.reference_used,
            });
        }

        let score_sum: f64 = run_scores.iter().sum();
        let score_count = run_scores.len() as i32;
        let run_avg = if score_count > 0 {
            score_sum / score_count as f64
        } else {
            0.0
        };

        println!(
            "   prompt_id={}  run_avg={:.3}  (sum={:.1} / count={})",
            prompt_id, run_avg, score_sum, score_count
        );

        per_prompt_scores.insert(prompt_id.clone(), run_avg);
        persist_prompt_eval_stats(&pool, prompt_id, score_sum, score_count).await?;
    }

    let overall_avg = if all_scores.is_empty() {
        0.0
    } else {
        all_scores.iter().sum::<f64>() / all_scores.len() as f64
    };

    println!(
        "\n📦 run_id={}  total_scored={}  overall_avg={:.3}",
        run_id,
        all_scores.len(),
        overall_avg
    );

    save_evaluation_run(
        &pool,
        &run_id,
        &dataset.id,
        &payload.prompt_ids,
        overall_avg,
        questions.len() as i32,
        &per_prompt_scores,
        created_at,
    )
    .await?;

    save_evaluation_details(&pool, &all_details).await?;

    println!(
        "✨ Evaluation complete  run_id={}  details={}",
        run_id,
        all_details.len()
    );

    Ok(Json(EvaluationResult {
        id: run_id,
        average_score: overall_avg,
        total_items: all_scores.len() as i32,
        scores: all_scores,
        dataset: dataset.name,
        prompts: payload.prompt_ids,
        per_prompt_scores,
        created_at,
    }))
}

#[derive(sqlx::FromRow)]
struct EvalRunListRow {
    id: String,
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
) -> Result<Json<Vec<EvaluationResult>>, StatusCode> {
    println!("📋 Listing evaluations");

    let rows = sqlx::query_as::<_, EvalRunListRow>(
        r#"
        SELECT
            er.id,
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
        GROUP BY er.id, er.prompt_ids, er.average_score, er.total_questions,
                 er.created_at, er.per_prompt_scores, d.name
        ORDER BY er.created_at DESC
        LIMIT 50
        "#,
    )
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
    Path(id): Path<String>,
) -> Result<Json<EvaluationWithDetails>, StatusCode> {
    println!("🔍 Getting evaluation: {}", id);

    let run = sqlx::query_as::<_, EvalRunRow>(
        r#"
        SELECT id, dataset_id, prompt_ids, average_score, total_questions,
               per_prompt_scores, created_at
        FROM evaluation_runs
        WHERE id = $1
        "#,
    )
    .bind(&id)
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
    Json(payload): Json<GenerateTestCasesRequest>,
) -> Result<Json<GenerateTestCasesResponse>, StatusCode> {
    let count = payload.count.clamp(1, 20);
    let prompt = load_prompt_context(&pool, &payload.prompt_id).await?;

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

    let make_prompt = |retry: bool| -> String {
        if retry {
            // Stripped-down retry prompt when the model produced invalid JSON.
            format!(
                "Return ONLY minified JSON array, no prose, no markdown. \
                 Exactly {} items. Each item: \
                 {{\"variable_values\":{{...}},\"expected_answer\":\"...\",\
                 \"difficulty\":\"medium\",\"case_type\":\"happy_path\",\
                 \"tags\":[\"...\"],\"reasoning\":\"...\"}}. \
                 Variables: {:?}",
                count, prompt.variables
            )
        } else {
            {
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

                format!(
                    r#"You are a test-case engineer generating evaluation data for an LLM prompt.

DOMAIN: {}
EXPECTED OUTPUT FORMAT: {}

PROMPT TEMPLATE:
{}

{}

EVALUATION RUBRIC:
{}{}

Generate {} test cases that comprehensively exercise this prompt.

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

Return ONLY a valid JSON array (no markdown fences, no prose):
[{{
  "variable_values": {{{{}}}},
  "expected_answer": "...",
  "difficulty": "easy|medium|hard|adversarial",
  "case_type": "...",
  "tags": ["dimension_tested:criterion_name", "difficulty:level"],
  "reasoning": "one sentence: what failure mode or success criterion this tests"
}}]"#,
                    domain,
                    output_format,
                    prompt.template,
                    variable_instruction,
                    rubric_text,
                    kb_section,
                    count,
                )
            }
        }
    };

    let mut parsed: Option<Value> = None;

    // Two-pass: normal prompt first, then a stripped-down retry on JSON parse failure.
    for retry in [false, true] {
        let text = llm
            .send_text(llm.model_sonnet(), 4000, &make_prompt(retry), None)
            .await?;

        let cleaned = text
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        // Try full parse first; if that fails, try extracting the JSON array substring.
        let try_parsed = serde_json::from_str::<Value>(cleaned).or_else(|_| {
            let start = cleaned.find('[').ok_or_else(|| {
                serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "no array start",
                ))
            })?;
            let end = cleaned.rfind(']').ok_or_else(|| {
                serde_json::Error::io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "no array end",
                ))
            })?;
            serde_json::from_str(&cleaned[start..=end])
        });

        match try_parsed {
            Ok(v) => {
                parsed = Some(v);
                break;
            }
            Err(e) => {
                eprintln!("generate_test_cases parse error (retry={}): {}", retry, e);
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
        assert_eq!(output.dimension_scores["tone"].reasoning, "The response has a professional and friendly tone.");
    }
}
