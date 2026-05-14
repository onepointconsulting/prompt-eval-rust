# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Full architecture diagram:** `docs/architecture.md` — component map, all API routes, the three core workflows, and known gaps.

## Commands

**Backend (Rust, repo root):**
```bash
cargo run          # start API server on http://127.0.0.1:3001
cargo check        # fast compile check without linking
cargo build        # full build
cargo test         # run tests
cargo test <name>  # run a single test by name
```

**Frontend (Next.js, `prompt-eval-ui/`):**
```bash
bun install        # install dependencies
bun run dev        # start dev server on http://localhost:3000
bun run build      # production build
bun run lint       # ESLint
```

**Environment — create `.env` in repo root:**
```env
ANTHROPIC_API_KEY=...
ANTHROPIC_MODEL_HAIKU=claude-haiku-4-5-20251001
ANTHROPIC_MODEL_SONNET=claude-sonnet-4-20250514
DATABASE_URL=postgresql://postgres:password@localhost:5432/prompt_eval
```
All three model vars are required — `AnthropicClient::from_env()` panics at startup if any are missing.

**Database setup:** Run `docs/db-queries.sql` against a PostgreSQL instance. Note: the file has a syntax error (missing comma on line 21 after `created_at`) — fix before running.

## Architecture

### State model (critical to understand before adding any handler)

Shared state lives in `src/app_state.rs`:
```rust
pub struct AppState { pub pool: PgPool, pub llm: AnthropicClient }
```
`FromRef<AppState>` is implemented for both `PgPool` and `AnthropicClient`. This lets each handler declare only what it needs:
- `State<PgPool>` — DB-only handlers (list, get, create, update, delete)
- `State<AnthropicClient>` — LLM-only handlers (generate_prompt)
- Both together — `run_evaluation`, `generate_test_cases`

When you add a new module, declare it in **both** `src/main.rs` (`mod foo;`) and `src/lib.rs` (`pub mod foo;`) — they are separate compilation units (binary crate vs. library crate) that both compile the same source files.

### LLM calls

All Anthropic calls go through `src/llm/anthropic_client.rs` — never use raw `reqwest` in handlers. The two key methods:
- `send_text(model, max_tokens, user_prompt, system_prompt)` → `Result<String, StatusCode>` — returns the extracted text content
- `send_json(...)` → `Result<Value, StatusCode>` — returns the full response JSON

Use `llm.model_haiku()` / `llm.model_sonnet()` to reference the configured model names rather than hardcoding strings.

### Evaluation pipeline

`POST /api/evaluate` → `handlers/evaluations::run_evaluation`:
1. Resolve dataset by `dataset_id` or `dataset_path` (legacy)
2. For each prompt_id × each question: fill template vars → Haiku generates response → Sonnet judges it (1–10 score)
3. Rolling-mean prompt stats update via `persist_prompt_eval_stats`
4. Persist `evaluation_runs` + `evaluation_details` rows

Template variables use `{{VAR_NAME}}` (double braces). Extraction: `utils/template_parser::extract_variables`. Substitution: `fill_template`. Questions store their variable bindings as `variable_values JSONB`.

### ID conventions

All IDs are timestamp-based strings generated at creation time:
- Datasets: `ds_{unix_timestamp}`
- Prompts: `p_{unix_timestamp}`
- Evaluation runs: `eval_{unix_timestamp}`
- Questions use a `SERIAL` integer PK

### Known stubs

- `src/handlers/stats.rs` — `GET /api/stats` returns hardcoded values; needs real DB aggregation queries
- `evaluation_details.strengths` / `weaknesses` columns exist in DB but are never populated by the judge
- `questions.expected_answer` is stored but ignored during evaluation

### Frontend notes

The frontend (`prompt-eval-ui/`) calls the backend at `http://localhost:3001`. The API client is `prompt-eval-ui/src/lib/api.ts` — all fetch calls go through typed wrappers there.

This frontend uses a non-standard Next.js build. Read `prompt-eval-ui/node_modules/next/dist/docs/` before writing any Next.js-specific code; APIs and conventions may differ from standard Next.js documentation.
