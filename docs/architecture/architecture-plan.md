# Evaluation System — Architectural Plan

> Analysis of the four core subsystems: judge, evaluation pipeline, prompt generation, and test case generation. Covers what is wrong, the correct redesign, schema changes, and implementation order.

---

## The Core Problem: A Broken Pipeline

The four areas form a chain where each output feeds the next. Every link is currently broken:

```
Prompt Generation  →  Test Cases  →  Evaluation Run  →  Judge
   (no rubric)       (no expected     (ignores            (hardcoded
   (Haiku, wrong      answers, no      expected_answer,    customer support,
    model)            difficulty)      10 token budget)    no rubric)
```

Fix the judge without the rubric and it is still generic. Fix the rubric without fixing test case generation and you have no ground truth. The plan below defines the correct order.

---

## Three Fundamental Design Decisions

Everything flows from these three choices:

**1. Rubric belongs on the Prompt, not the Evaluator.**
The rubric captures the prompt author's intent — what does "good" mean for *this* prompt? It gets generated once when the prompt is created, stored in the DB, and passed to the judge at evaluation time. A coding assistant prompt and a teaching assistant prompt have completely different rubrics.

**2. Score is a vector, not a scalar.**
A single 7.2/10 is nearly useless. "7.2 because accuracy=9, clarity=4" tells you what to fix. The DB already has `strengths` and `weaknesses` columns — they are just never populated. The judge must return multi-dimensional output.

**3. Expected answers are semantic specifications, not verbatim strings.**
`expected_answer` should not be the literal correct text — it should describe what a correct response *must contain*. Example: "Must ask exactly one question, in English, without revealing the answer." This is robust to rephrasing and gives the judge concrete criteria to verify against.

---

## Diagnosis

### Judge Response

**Fundamentally wrong:**

- Hardcoded customer support rubric. Scores every prompt — teaching assistants, code reviewers, legal summarisers — against "was it professional and did it provide clear next steps?"
- `max_tokens: 10` for the judge call. Structured multi-dimensional output with reasoning cannot fit in 10 tokens. Scores are inconsistent because the model has no room to reason before committing to a number.

**Incomplete:**

- `strengths` and `weaknesses` columns exist in `evaluation_details` but are never populated. The schema was designed for richer output; the implementation never got there.
- `expected_answer` exists on every question row but never reaches the judge. It is selected nowhere in `load_questions_with_ids` and is absent from the `EvalQuestion` struct.

### Evaluation Architecture

**Fundamentally wrong:**

- Rolling mean math is incorrect. `persist_prompt_eval_stats` computes `(old_avg * runs + new_run_avg) / (runs + 1)` — a weighted mean of means. This is only accurate when all runs have the same number of questions. A run with 2 questions and a run with 20 questions receive equal weight, producing a wrong aggregate.

**Incomplete:**

- No per-prompt scores on the run record. `evaluation_runs.average_score` collapses all prompts into one number. The frontend has to recompute per-prompt scores from `evaluation_details` joins — it already does this in `api.ts`. The schema is making the frontend compensate for a server-side deficiency.
- No async execution. With 5 prompts × 20 questions = 100 sequential API calls at 3–5 seconds each, `POST /api/evaluate` will block for 5–8 minutes and time out. The evaluation must be accepted as a job and executed in the background.
- No baseline/control prompt. A score of 7.2 is meaningless without context. If the simplest possible prompt also scores 7.1, the complex prompt is providing no value.

### Prompt Template Generation

**Fundamentally wrong:**

- Using Haiku for a reasoning-heavy task. Generating a well-structured prompt template requires understanding domain, intent, and output constraints. Haiku produces syntactically valid templates that are semantically weak. The template is the root of the evaluation chain — a low-quality template poisons every downstream result.
- Single-shot generation with no system prompt, no structured output, no validation that `{{VAR}}` placeholders in the output match what was declared.

**Incomplete:**

- No rubric generated alongside the template. The prompt knows its purpose but that knowledge is thrown away. The rubric must be emitted during generation and stored on the `prompts` row.
- No domain, output format, or variable semantics captured. Test case generation and the judge both receive the raw template with no context about what kind of task it performs.

### Test Case Generation

**Fundamentally wrong:**

- No `expected_answer` generated. Without ground truth, the judge operates in reference-free mode for every test case. Reference-free LLM evaluation has well-documented reliability problems.
- Tags are semantically empty. The generator produces tags like `["customer_support", "billing"]` with no enforced schema and no downstream query that uses them. Tags should encode `difficulty`, `case_type` (happy path, edge case, adversarial), and the rubric dimension being probed.

**Incomplete:**

- No difficulty stratification. All cases are "diverse" surface variations of normal inputs. Without easy/medium/hard/adversarial distribution, you cannot distinguish a prompt that fails on hard cases from one that fails on easy cases.
- Using Haiku instead of Sonnet. Given rubric and domain context, Sonnet produces test cases that deliberately probe each rubric dimension. Haiku produces generic variations.

---

## Correct Architecture Per Subsystem

### Judge Response

The judge must accept:

1. The original question
2. The model's response
3. The evaluation rubric (2–5 criteria specific to this prompt)
4. Optionally: `expected_answer` as a semantic specification

The judge must return structured JSON:

```json
{
  "dimension_scores": {
    "criterion_name": { "score": 8, "reasoning": "..." }
  },
  "strengths": ["specific thing done well"],
  "weaknesses": ["specific thing done poorly"],
  "overall_score": 7.0,
  "judge_reasoning": "full chain-of-thought before scoring",
  "reference_used": true
}
```

**When no rubric exists:** Fall back to four universal dimensions — relevance, accuracy, completeness, clarity. These are domain-agnostic and do not impose customer support assumptions.

**G-Eval pattern:** Ask the judge to produce its reasoning *before* the numeric scores, not after. This produces more consistent scores because the model cannot commit to a number before thinking. The current pattern (asking for a bare number with `max_tokens: 10`) produces arbitrary scores.

**Token budget:** Minimum 800 tokens. Structured JSON with reasoning cannot fit in less.

### Evaluation Architecture

**Fix the rolling mean.** Replace the single `average_score` float with two counters: `total_score_sum` and `total_score_count`. The correct average is always `total_score_sum / total_score_count`. The update adds `question_count` many individual scores, not a run average. This is correct regardless of run size.

**Per-prompt scores as first-class data.** `evaluation_runs` should store a `per_prompt_scores JSONB` column: `{"p_123": 7.4, "p_456": 6.1}`. This eliminates frontend recomputation from `evaluation_details` joins.

**Async execution.** `POST /api/evaluate` should: create the run row with `status = 'pending'`, spawn a `tokio::spawn` background task, and return the run ID immediately. The background task transitions status to `running` → `completed` or `failed`. The existing `GET /api/evaluations/:id` endpoint already returns status — clients poll it.

**Baseline prompt.** The run should accept an optional `baseline_prompt_id`. If provided, per-prompt scores are reported as absolute and as relative improvement over baseline. This gives scores meaning beyond their raw value.

### Prompt Template Generation

Two changes: model upgrade and structured output.

**Use Sonnet.** The template is the root of the evaluation chain. This is the one place where quality matters most and cost is least important (templates are generated once, not on every evaluation call).

**Request structured JSON, not raw text.** The generation prompt should have a system prompt instructing Sonnet to produce:

```json
{
  "template": "You are a teaching mentor...",
  "variables": [
    { "name": "STUDENT_PROFILE", "description": "what this variable contains" }
  ],
  "domain": "educational_assistant",
  "rubric": [
    { "name": "socratic_adherence", "description": "Did it ask exactly one guiding question?", "weight": 0.4 }
  ],
  "expected_output_format": "single conversational question in the student's language"
}
```

**Validate after generation.** Run `extract_variables` on the template and verify every declared variable appears as a `{{VAR}}` placeholder. If they do not match, return a validation error rather than storing a broken template.

### Test Case Generation

**Output shape per generated test case:**

```json
{
  "variable_values": { "STUDENT_PROFILE": "...", "STUDENT_MESSAGE": "..." },
  "expected_answer": "Must acknowledge frustration, ask exactly one question about what the student sees in the output, and respond in English.",
  "difficulty": "hard",
  "case_type": "emotional_stress",
  "tags": ["emotional_signal:frustrated", "language:english", "domain:javascript"],
  "reasoning": "Tests whether the prompt correctly prioritises emotional acknowledgment before technical guidance when the student expresses distress."
}
```

**Difficulty distribution.** For a count of 10 test cases: 3 easy (straightforward happy-path), 4 medium (typical with some complexity), 2 hard (ambiguous or multi-layered), 1 adversarial (designed to expose a failure mode). Specify this distribution explicitly in the generator prompt and require it in the JSON output schema.

**Use Sonnet.** Given the prompt's rubric and domain, Sonnet can generate test cases that deliberately probe each rubric dimension. This is impossible with Haiku without domain context.

**Feed in rubric and domain.** The generator must load `rubric`, `domain`, and `expected_output_format` from the prompt row before calling the LLM. Without these, generated test cases are generic regardless of model quality.

---

## Schema Changes

Apply these in order. All new columns have defaults — no existing queries break.

```sql
-- 1. Prompts: rubric and domain context
ALTER TABLE prompts
  ADD COLUMN domain VARCHAR(100),
  ADD COLUMN rubric JSONB,
  ADD COLUMN expected_output_format TEXT;

-- 2. Prompts: fix the rolling mean — replace single float with counters
ALTER TABLE prompts
  ADD COLUMN total_score_sum DOUBLE PRECISION NOT NULL DEFAULT 0,
  ADD COLUMN total_score_count INTEGER NOT NULL DEFAULT 0;
-- average_score is now total_score_sum / NULLIF(total_score_count, 0)
-- can keep the column as a denormalised cache if needed, but counters are authoritative

-- 3. Questions: difficulty and case metadata
ALTER TABLE questions
  ADD COLUMN difficulty VARCHAR(20) DEFAULT 'medium',   -- easy | medium | hard | adversarial
  ADD COLUMN case_type VARCHAR(50),                     -- happy_path | edge_case | emotional_stress | etc.
  ADD COLUMN reasoning TEXT;                            -- why this case tests what it tests

-- 4. Evaluation details: activate dead columns and add new ones
ALTER TABLE evaluation_details
  ADD COLUMN dimension_scores JSONB,
  ADD COLUMN judge_reasoning TEXT,
  ADD COLUMN reference_used BOOLEAN DEFAULT FALSE;
-- strengths TEXT[] and weaknesses TEXT[] already exist, just need to be populated

-- 5. Evaluation runs: per-prompt breakdown and baseline
ALTER TABLE evaluation_runs
  ADD COLUMN per_prompt_scores JSONB,
  ADD COLUMN baseline_prompt_id VARCHAR(50);

-- 6. Indexes for new query patterns
CREATE INDEX idx_evaluation_details_prompt ON evaluation_details(prompt_id);
CREATE INDEX idx_evaluation_details_run_prompt ON evaluation_details(run_id, prompt_id);
```

---

## Implementation Order

The order is determined by the dependency chain — each step unlocks the next.


| Step  | What                                                                                                                                   | Key files                                      |
| ----- | -------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| **1** | Schema migrations                                                                                                                      | `docs/db-queries.sql`, run against DB          |
| **2** | Fix the judge — structured output, correct token budget, generic fallback rubric, populate `strengths`/`weaknesses`/`dimension_scores` | `src/handlers/evaluations.rs`, `src/models.rs` |
| **3** | Fix rolling mean — replace with `total_score_sum` + `total_score_count` counters                                                       | `src/handlers/evaluations.rs`                  |
| **4** | Wire `expected_answer` — add to `EvalQuestion`, thread through loop to judge                                                           | `src/handlers/evaluations.rs`                  |
| **5** | Upgrade prompt generation — Sonnet, structured JSON, emit rubric + domain, validate variables                                          | `src/handlers/prompts.rs`, `src/models.rs`     |
| **6** | Upgrade test case generation — load rubric/domain, emit `expected_answer` + difficulty + `case_type`, Sonnet                           | `src/handlers/evaluations.rs`, `src/models.rs` |
| **7** | Wire rubric into the evaluation loop — load from prompt row, pass to judge                                                             | `src/handlers/evaluations.rs`                  |
| **8** | Add `per_prompt_scores` to run record — compute and persist the per-prompt breakdown                                                   | `src/handlers/evaluations.rs`, `src/models.rs` |
| **9** | Make evaluation async — spawn background task, return run ID immediately, poll for status                                              | `src/handlers/evaluations.rs`, `src/routes.rs` |


Steps 1–4 can be done without any user-visible behaviour change and are safe to ship incrementally. Steps 5–7 form a unit that should land together. Steps 8–9 are independent improvements.

---

## What Stays Exactly As-Is


| Component                                             | Why                                                         |
| ----------------------------------------------------- | ----------------------------------------------------------- |
| `AnthropicClient` in `src/llm/`                       | Clean abstraction, correct primitives, env-var model config |
| `AppState` / `FromRef` pattern                        | Idiomatic Axum sub-state extraction                         |
| `{{VAR_NAME}}` syntax and `template_parser` utilities | Correct and used consistently everywhere                    |
| `resolve_dataset` dual-path logic                     | id-preferred, name-fallback is correct backward-compat      |
| Transaction-wrapped dataset upload in `datasets.rs`   | Atomicity is correct                                        |
| JSON parse retry pattern in test case generation      | Pragmatic and works — keep it, extend it to judge output    |
| Frontend API client structure in `api.ts`             | Typed wrappers with snake_case→camelCase isolation is good  |


