-- =============================================================================
-- Prompt Evaluation System — Database Schema
-- =============================================================================
-- Run this file once against a fresh PostgreSQL database.
-- For existing databases, see the MIGRATIONS section at the bottom.
-- =============================================================================


-- -----------------------------------------------------------------------------
-- datasets
-- Metadata for a named collection of questions used in evaluations.
-- question_count is denormalised and kept in sync manually on every
-- question insert/delete — it avoids a COUNT(*) on hot read paths.
-- -----------------------------------------------------------------------------
CREATE TABLE datasets (
    id              VARCHAR(50)  PRIMARY KEY,           -- format: ds_{unix_timestamp}
    name            VARCHAR(255) NOT NULL,
    description     TEXT,
    question_count  INTEGER      NOT NULL DEFAULT 0,
    avg_score       DOUBLE PRECISION,                   -- running average across all eval runs
    evaluations     INTEGER      DEFAULT 0,             -- number of times this dataset has been evaluated
    created_by      VARCHAR(100),
    created_at      TIMESTAMPTZ  DEFAULT NOW()
);


-- -----------------------------------------------------------------------------
-- questions
-- Individual test inputs that belong to a dataset.
--
-- variable_values  JSONB: {"VAR_NAME": "value", ...}
--   Used when the associated prompt is templated (is_templated = true).
--   The evaluation pipeline calls fill_template(prompt.template, variable_values).
--
-- expected_answer  TEXT (optional):
--   A semantic specification of what a correct response must contain — NOT a
--   verbatim answer. Example: "Must ask exactly one question in English without
--   revealing the solution." Passed to the LLM judge as a reference.
--
-- difficulty       VARCHAR: easy | medium | hard | adversarial
--   easy        → straightforward happy-path input
--   medium      → typical usage with moderate complexity
--   hard        → ambiguous, multi-layered, or edge-case input
--   adversarial → input designed to probe a specific failure mode
--
-- case_type        VARCHAR: free-form label, e.g. happy_path, edge_case,
--                  emotional_stress, out_of_scope, boundary_value
--
-- reasoning        TEXT: why this test case is useful — which rubric dimension
--                  it probes and what failure mode it exercises.
-- -----------------------------------------------------------------------------
CREATE TABLE questions (
    id              SERIAL       PRIMARY KEY,
    dataset_id      VARCHAR(50)  NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    question_text   TEXT         NOT NULL,
    expected_answer TEXT,                               -- semantic spec, not verbatim
    question_order  INTEGER,
    variable_values JSONB,                              -- {"VAR": "value"} for templated prompts
    tags            TEXT[],                             -- arbitrary labels, e.g. ["topic:billing"]
    difficulty      VARCHAR(20)  DEFAULT 'medium',      -- easy | medium | hard | adversarial
    case_type       VARCHAR(50),                        -- happy_path | edge_case | etc.
    reasoning       TEXT,                               -- why this case probes what it probes
    created_at      TIMESTAMPTZ  DEFAULT NOW()
);


-- -----------------------------------------------------------------------------
-- prompts
-- Prompt templates evaluated by the system.
--
-- template         TEXT:
--   The prompt body. May contain {{VAR_NAME}} placeholders (double braces).
--   Placeholders are extracted by utils::template_parser::extract_variables
--   and filled per-question by fill_template.
--
-- variables        TEXT[]: declared placeholder names, e.g. {"QUESTION", "CONTEXT"}
-- is_templated     BOOL: true when the template has at least one {{VAR}} placeholder
--
-- rubric           JSONB: array of RubricCriterion objects:
--   [{"name": "relevance", "description": "...", "weight": 0.25}, ...]
--   Weights must sum to 1.0. Passed to the LLM judge at evaluation time.
--   If NULL, the judge falls back to a generic four-dimension rubric
--   (relevance, accuracy, completeness, clarity).
--
-- domain           VARCHAR: short snake_case task category,
--   e.g. educational_assistant, code_review, customer_support, medical_qa.
--   Used to contextualise test case generation.
--
-- expected_output_format  TEXT:
--   Human-readable description of what ideal output looks like.
--   e.g. "A single conversational question in the student's language."
--   Passed to the test case generator so it can produce relevant expected_answers.
--
-- runs             INTEGER: number of evaluation runs this prompt has participated in.
-- average_score    DOUBLE PRECISION: denormalised mean — kept in sync via
--   total_score_sum / total_score_count so it is always accurate regardless of
--   whether runs had equal numbers of questions (the rolling-mean-of-means bug).
-- total_score_sum  DOUBLE PRECISION: sum of ALL individual question scores ever recorded.
-- total_score_count INTEGER: count of individual scores, i.e. denominator for the mean.
-- -----------------------------------------------------------------------------
CREATE TABLE prompts (
    id                      VARCHAR(50)  PRIMARY KEY,  -- format: p_{unix_timestamp}
    name                    VARCHAR(255) NOT NULL,
    template                TEXT         NOT NULL,
    variables               TEXT[],                    -- extracted {{VAR}} placeholder names
    is_templated            BOOLEAN      DEFAULT FALSE,
    status                  VARCHAR(50)  NOT NULL DEFAULT 'draft', -- draft | active | archived
    runs                    INTEGER      NOT NULL DEFAULT 0,
    updated_at              TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    average_score           DOUBLE PRECISION,          -- = total_score_sum / total_score_count
    total_score_sum         DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_score_count       INTEGER      NOT NULL DEFAULT 0,
    domain                  VARCHAR(100),              -- task category label
    rubric                  JSONB,                     -- [{name, description, weight}]
    expected_output_format  TEXT                       -- what ideal output looks like
);


-- -----------------------------------------------------------------------------
-- evaluation_runs
-- One record per execution of POST /api/evaluate.
-- A single run can test multiple prompts against the same dataset (A/B style).
--
-- prompt_ids         TEXT[]: the prompt IDs tested in this run.
-- average_score      DOUBLE PRECISION: collapsed mean across all (prompt × question) scores.
-- per_prompt_scores  JSONB: per-prompt breakdown — {"p_123": 7.4, "p_456": 6.1}
--   Eliminates the need for the frontend to recompute from evaluation_details joins.
-- baseline_prompt_id VARCHAR: one of the prompt_ids nominated as the control.
--   Scores for other prompts should be interpreted relative to this baseline.
-- status             VARCHAR: pending | running | completed | failed
--   Currently always written as 'completed'. Reserved for async execution (Step 9).
-- -----------------------------------------------------------------------------
CREATE TABLE evaluation_runs (
    id                  VARCHAR(50)  PRIMARY KEY,      -- format: eval_{unix_timestamp}
    dataset_id          VARCHAR(50)  REFERENCES datasets(id),
    prompt_ids          TEXT[]       NOT NULL,
    average_score       DOUBLE PRECISION NOT NULL,
    total_questions     INTEGER      NOT NULL,
    status              VARCHAR(50)  DEFAULT 'completed',
    per_prompt_scores   JSONB,                         -- {"prompt_id": score}
    baseline_prompt_id  VARCHAR(50),                   -- optional control prompt
    created_at          TIMESTAMPTZ  DEFAULT NOW()
);


-- -----------------------------------------------------------------------------
-- evaluation_details
-- One record per (run, prompt, question) triple — the atomic result unit.
--
-- model_answer       TEXT: the raw text extracted from the LLM response.
--   NOTE: this is the text content only, not the full Anthropic API JSON envelope.
--
-- score              DOUBLE PRECISION: overall judge score (1.0–10.0).
--
-- dimension_scores   JSONB: per-criterion scores from the judge:
--   {"relevance": {"score": 8, "reasoning": "..."}, "accuracy": {"score": 6, ...}}
--   Stored as JSONB so new rubric dimensions never require schema changes.
--
-- strengths          TEXT[]: concrete things the response did well (from judge).
-- weaknesses         TEXT[]: concrete things the response did poorly (from judge).
-- judge_reasoning    TEXT: the judge's chain-of-thought before assigning scores.
--   Valuable for debugging unexpected scores.
-- reference_used     BOOL: true when expected_answer was available and incorporated
--   into the judge's assessment.
-- -----------------------------------------------------------------------------
CREATE TABLE evaluation_details (
    id              SERIAL       PRIMARY KEY,
    run_id          VARCHAR(50)  REFERENCES evaluation_runs(id) ON DELETE CASCADE,
    question_id     INTEGER      REFERENCES questions(id),
    prompt_id       VARCHAR(50),
    model_answer    TEXT,
    score           DOUBLE PRECISION,
    dimension_scores    JSONB,                         -- {"criterion": {"score":n, "reasoning":"..."}}
    strengths       TEXT[],
    weaknesses      TEXT[],
    judge_reasoning TEXT,
    reference_used  BOOLEAN      DEFAULT FALSE,
    created_at      TIMESTAMPTZ  DEFAULT NOW()
);


-- -----------------------------------------------------------------------------
-- Indexes
-- -----------------------------------------------------------------------------
CREATE INDEX idx_prompts_status        ON prompts(status);
CREATE INDEX idx_prompts_updated       ON prompts(updated_at DESC);
CREATE INDEX idx_questions_dataset     ON questions(dataset_id);
CREATE INDEX idx_questions_difficulty  ON questions(dataset_id, difficulty); -- filter by difficulty tier
CREATE INDEX idx_eval_runs_dataset     ON evaluation_runs(dataset_id);
CREATE INDEX idx_eval_details_run      ON evaluation_details(run_id);
CREATE INDEX idx_eval_details_prompt   ON evaluation_details(prompt_id);
CREATE INDEX idx_eval_details_run_prompt ON evaluation_details(run_id, prompt_id); -- per-prompt score query


-- =============================================================================
-- MIGRATIONS
-- Run these against an existing database that was created with the old schema.
-- Safe to run multiple times (use IF NOT EXISTS / DO $$ guards where possible).
-- =============================================================================

-- prompts: rubric system + accurate rolling mean
ALTER TABLE prompts
    ADD COLUMN IF NOT EXISTS total_score_sum        DOUBLE PRECISION NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS total_score_count      INTEGER          NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS domain                 VARCHAR(100),
    ADD COLUMN IF NOT EXISTS rubric                 JSONB,
    ADD COLUMN IF NOT EXISTS expected_output_format TEXT;

-- NOTE: existing rows will have total_score_sum=0, total_score_count=0.
-- The stored average_score becomes the authoritative value for those rows
-- until the next evaluation run recalculates from live scores.

-- questions: difficulty and case metadata
ALTER TABLE questions
    ADD COLUMN IF NOT EXISTS difficulty  VARCHAR(20) DEFAULT 'medium',
    ADD COLUMN IF NOT EXISTS case_type   VARCHAR(50),
    ADD COLUMN IF NOT EXISTS reasoning   TEXT;

-- evaluation_details: activate dead columns + add judge output fields
ALTER TABLE evaluation_details
    ADD COLUMN IF NOT EXISTS dimension_scores  JSONB,
    ADD COLUMN IF NOT EXISTS judge_reasoning   TEXT,
    ADD COLUMN IF NOT EXISTS reference_used    BOOLEAN DEFAULT FALSE;

-- evaluation_runs: per-prompt breakdown
ALTER TABLE evaluation_runs
    ADD COLUMN IF NOT EXISTS per_prompt_scores   JSONB,
    ADD COLUMN IF NOT EXISTS baseline_prompt_id  VARCHAR(50);

-- New indexes (IF NOT EXISTS not supported for indexes in older PG — check first)
CREATE INDEX IF NOT EXISTS idx_questions_difficulty    ON questions(dataset_id, difficulty);
CREATE INDEX IF NOT EXISTS idx_eval_details_prompt     ON evaluation_details(prompt_id);
CREATE INDEX IF NOT EXISTS idx_eval_details_run_prompt ON evaluation_details(run_id, prompt_id);

-- prompts: knowledge base context configuration
ALTER TABLE prompts
    ADD COLUMN IF NOT EXISTS use_context      BOOLEAN      NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS context_project  VARCHAR(100);
