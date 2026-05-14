# Prompt Evaluation System — Architecture

---

## Big Picture

```
╔══════════════════════════════════════════════════════════════════════════════════╗
║                        PROMPT EVALUATION SYSTEM                                  ║
╠══════════════════════════════════════════════════════════════════════════════════╣
║                                                                                  ║
║   ┌─────────────────────┐      HTTP / JSON       ┌───────────────────────────┐  ║
║   │   NEXT.JS FRONTEND  │ ◄───────────────────► │   RUST / AXUM BACKEND     │  ║
║   │   localhost:3000    │                        │   127.0.0.1:3001          │  ║
║   └─────────────────────┘                        └──────────────┬────────────┘  ║
║                                                                  │              ║
║                                          ┌───────────────────────┤              ║
║                                          │                       │              ║
║                               ┌──────────▼──────────┐  ┌────────▼────────────┐ ║
║                               │     POSTGRESQL      │  │   ANTHROPIC  API    │ ║
║                               │     DATABASE        │  │                     │ ║
║                               └─────────────────────┘  └─────────────────────┘ ║
╚══════════════════════════════════════════════════════════════════════════════════╝
```

---

## System Components

### Frontend — Next.js 16 (`localhost:3000`)

```
prompt-eval-ui/
├── src/app/
│   ├── page.tsx                 →  Dashboard       (stats + trend chart)
│   ├── prompts/
│   │   ├── page.tsx             →  Prompt Library  (list, create, delete)
│   │   ├── [id]/page.tsx        →  Prompt Editor   (edit template, rubric, domain)
│   │   └── generate/page.tsx   →  AI Generator    (describe → generate prompt)
│   ├── datasets/
│   │   ├── page.tsx             →  Dataset List    (list, create, delete)
│   │   ├── [id]/page.tsx        →  Dataset Detail  (questions table, rename)
│   │   └── upload/page.tsx     →  Upload          (JSON upload)
│   ├── evaluate/page.tsx        →  Run Eval        (pick prompt + dataset → run)
│   ├── results/[id]/page.tsx    →  Eval Results    (per-question scores, judge)
│   └── history/page.tsx         →  Eval History    (all past runs)
│
└── src/lib/
    ├── api.ts                   →  Typed HTTP client (snake_case ↔ camelCase)
    └── types.ts                 →  Shared TypeScript types
```

### Backend — Rust / Axum (`127.0.0.1:3001`)

```
src/
├── main.rs           →  Entry point: wires DB pool + LLM client into AppState
├── lib.rs            →  Library crate (exposes modules for tests)
├── app_state.rs      →  AppState { pool: PgPool, llm: AnthropicClient }
│                        FromRef<AppState> for PgPool and AnthropicClient
├── routes.rs         →  All route registrations in one place
├── models.rs         →  All request/response structs (Serialize/Deserialize)
│
├── handlers/
│   ├── prompts.rs        →  CRUD + generate_prompt
│   ├── datasets.rs       →  CRUD + get_questions + add_question
│   ├── evaluations.rs    →  run_evaluation + generate_test_cases + list + get
│   └── stats.rs          →  GET /api/stats  (⚠ hardcoded — needs real DB queries)
│
├── llm/
│   ├── mod.rs
│   └── anthropic_client.rs  →  send_text() + send_json()
│                                model_haiku() / model_sonnet() accessors
│
└── utils/
    └── template_parser.rs   →  extract_variables() + fill_template()
                                 Parses {{VAR_NAME}} placeholders
```

### Database — PostgreSQL

```
┌──────────────┐        ┌──────────────────┐       ┌──────────────────────┐
│   prompts    │        │    datasets      │       │      questions       │
│──────────────│        │──────────────────│       │──────────────────────│
│ id  (p_ts)   │        │ id  (ds_ts)      │       │ id  SERIAL           │
│ name         │        │ name             │  1:N  │ dataset_id  ────────►│
│ template     │        │ description      │◄──────│ question_text        │
│ variables[]  │        │ question_count   │       │ expected_answer      │
│ is_templated │        │ avg_score        │       │ question_order       │
│ status       │        │ evaluations      │       │ variable_values JSONB│
│ domain       │        │ created_at       │       │ difficulty           │
│ rubric JSONB │        └──────────────────┘       │ case_type            │
│ avg_score    │                                    │ tags[]               │
│ runs         │                                    └──────────────────────┘
│ updated_at   │
└──────┬───────┘
       │
       │  evaluation_runs reference prompt IDs (stored as JSONB array)
       │
       ▼
┌──────────────────────┐       ┌──────────────────────────────┐
│   evaluation_runs    │  1:N  │     evaluation_details       │
│──────────────────────│◄──────│──────────────────────────────│
│ id  (eval_ts)        │       │ id  SERIAL                   │
│ prompt_ids[]         │       │ run_id  ────────────────────►│
│ dataset_name         │       │ prompt_id                    │
│ average_score        │       │ question_text                │
│ per_prompt_scores    │       │ response                     │
│ created_at           │       │ score  (1–10)                │
└──────────────────────┘       │ strengths[]                  │
                                │ weaknesses[]                 │
                                │ dimension_scores JSONB       │
                                │ judge_reasoning              │
                                │ reference_used               │
                                └──────────────────────────────┘
```

### AI Layer — Anthropic API

```
┌─────────────────────────────────────────────────┐
│               AnthropicClient                   │
│─────────────────────────────────────────────────│
│  model_haiku()   →  claude-haiku-4-5-20251001   │
│  model_sonnet()  →  claude-sonnet-4-20250514    │
│                                                 │
│  send_text(model, max_tokens, user, system)     │
│    └─ returns: String                           │
│                                                 │
│  send_json(model, max_tokens, user, system)     │
│    └─ returns: serde_json::Value                │
└─────────────────────────────────────────────────┘

  Haiku  →  response generation (cheap, fast, per question)
  Sonnet →  judge scoring  (structured JSON output, per question)
  Sonnet →  prompt generation (once per new prompt)
  Sonnet →  test case generation (once per generate request)
```

---

## API Routes

```
METHOD  PATH                              HANDLER                    NOTES
──────  ────────────────────────────────  ─────────────────────────  ──────────────────
GET     /api/stats                        stats::get                 ⚠ hardcoded values
GET     /api/prompts                      prompts::list
POST    /api/prompts                      prompts::create
POST    /api/prompts/generate             prompts::generate_prompt   Sonnet call
GET     /api/prompts/:id                  prompts::get
PUT     /api/prompts/:id                  prompts::update
DELETE  /api/prompts/:id                  prompts::delete

GET     /api/datasets                     datasets::list
POST    /api/datasets                     datasets::create
POST    /api/datasets/upload              datasets::upload
GET     /api/datasets/:id                 datasets::get
PUT     /api/datasets/:id                 datasets::update
DELETE  /api/datasets/:id                 datasets::delete
GET     /api/datasets/:id/questions       datasets::get_questions
POST    /api/datasets/:id/questions       datasets::add_question

POST    /api/evaluate                     evaluations::run_evaluation  Haiku + Sonnet
GET     /api/evaluations                  evaluations::list_evaluations
GET     /api/evaluations/:id              evaluations::get_evaluation

POST    /api/questions/generate           evaluations::generate_test_cases  Sonnet
```

---

## Three Core Workflows

### 1 — Create a Prompt (AI-assisted)

```
  User types description
         │
         ▼
  POST /api/prompts/generate
         │
         ▼
  ┌──────────────────────────────────────────────────────────┐
  │  Sonnet generates structured JSON:                        │
  │  { template, variables[], domain, rubric[], output_fmt } │
  └──────────────────────────────────────────────────────────┘
         │
         ▼
  Frontend shows preview
  User clicks Save
         │
         ▼
  POST /api/prompts  →  stored in prompts table
```

---

### 2 — Generate Test Cases (AI-assisted)

```
  User picks a prompt
         │
         ▼
  POST /api/questions/generate  { prompt_id, count }
         │
         ▼
  evaluations.rs loads prompt (template + rubric + domain)
         │
         ▼
  ┌──────────────────────────────────────────────────────────────┐
  │  Sonnet generates N test cases, each with:                    │
  │  { variable_values, expected_answer, difficulty,             │
  │    case_type, tags[], reasoning }                            │
  └──────────────────────────────────────────────────────────────┘
         │
         ▼
  Stored as questions rows in the chosen dataset
```

---

### 3 — Run an Evaluation (the main pipeline)

```
  User picks prompt(s) + dataset
         │
         ▼
  POST /api/evaluate  { prompt_ids[], dataset_id }
         │
         ▼
  Load dataset questions from DB
         │
         ▼
  ┌─ For each prompt_id × each question ──────────────────────────────────┐
  │                                                                        │
  │   1. Fill template vars                                                │
  │      fill_template(template, question.variable_values)                │
  │             │                                                          │
  │             ▼                                                          │
  │   2. Generate response                                                 │
  │      Haiku  ←  filled prompt + question_text                          │
  │      Haiku  →  response text                                           │
  │             │                                                          │
  │             ▼                                                          │
  │   3. Judge the response                                                │
  │      Sonnet ←  question + response + rubric + expected_answer         │
  │      Sonnet →  { dimension_scores, strengths, weaknesses,             │
  │                  overall_score, judge_reasoning, reference_used }     │
  │             │                                                          │
  │             ▼                                                          │
  │   4. Persist evaluation_details row                                   │
  │                                                                        │
  └────────────────────────────────────────────────────────────────────────┘
         │
         ▼
  Compute average scores (per-prompt + overall)
  Persist evaluation_runs row
  Update prompts.avg_score rolling mean
         │
         ▼
  Return EvaluationResult  →  frontend renders results page
```

---

## Axum State Model

This is the Rust-specific design that lets each handler declare only what it needs:

```
                    AppState
                 ┌─────────────┐
                 │  pool       │  ← PgPool (database connection pool)
                 │  llm        │  ← AnthropicClient
                 └──────┬──────┘
                        │  FromRef<AppState> implemented for both
                        │
          ┌─────────────┼─────────────┐
          │             │             │
   State<PgPool>  State<AnthropicClient>  Both together
          │             │             │
   DB-only handlers   LLM-only      run_evaluation
   list, get, create  generate_prompt  generate_test_cases
   update, delete
```

**Why this matters:** When adding a new handler, declare only the state it needs. If it touches the DB, use `State<PgPool>`. If it calls Anthropic, use `State<AnthropicClient>`. Never use the full `State<AppState>` — it couples the handler to state it doesn't use.

---

## Module Registration Rule

Every new source file must be declared in **both** `src/main.rs` and `src/lib.rs`:

```
src/main.rs       →  mod foo;        (binary crate — runs the server)
src/lib.rs        →  pub mod foo;    (library crate — used by tests)
```

They compile the same source files separately. Missing a declaration in either causes a compile error.

---

## ID Conventions

```
Datasets          →  ds_{unix_timestamp}     e.g. ds_1778705698
Prompts           →  p_{unix_timestamp}      e.g. p_1778705527
Evaluation runs   →  eval_{unix_timestamp}   e.g. eval_1778741855
Questions         →  SERIAL integer (auto-increment)
```

---

## Known Gaps (not yet implemented)

| Area | Status | File |
|---|---|---|
| `GET /api/stats` | Returns hardcoded values | `src/handlers/stats.rs` |
| Async evaluation | Runs synchronously — will timeout on large datasets | `src/handlers/evaluations.rs` |
| Rolling mean accuracy | Weighted mean of means (wrong for unequal run sizes) | `src/handlers/evaluations.rs` |
| Model selection from frontend | Models hardcoded in backend | `src/handlers/evaluations.rs` |
| Export functionality | UI exists, no backend | — |
| Search / filter | UI inputs exist, no logic | — |
