# Testing Plan — Rust Backend

## 1. Dependencies to Add

Add to `Cargo.toml`:

```toml
[dev-dependencies]
# axum handler testing without spinning up a real server
tower = { version = "0.4", features = ["util"] }
http-body-util = "0.1"

# HTTP mock server — intercepts reqwest calls to Anthropic + context engine
wiremock = "0.6"

# isolated test databases per test (creates + drops a fresh DB automatically)
sqlx = { version = "0.8", features = ["test-utils", "postgres", "runtime-tokio-rustls", "chrono", "json"] }
```

No new test framework needed — `#[tokio::test]` is already available from `tokio = { features = ["full"] }`.

---

## 2. Migration File

`#[sqlx::test]` requires migrations in a `migrations/` folder with numbered files.
Extract `docs/db-queries.sql` into:

```
migrations/
  0001_initial_schema.sql   ← contents of db-queries.sql (fix the known comma bug on line 21)
```

---

## 3. Unit Tests

Pure functions — no DB, no network. Go in `#[cfg(test)]` modules at the bottom of each source file.

### `src/utils/template_parser.rs`


| Test                              | What it checks                            |
| --------------------------------- | ----------------------------------------- |
| `extract_variables_double_braces` | `{{VAR}}` correctly extracted             |
| `extract_variables_no_vars`       | plain text returns empty vec              |
| `extract_variables_multiple`      | multiple vars all captured, no duplicates |
| `fill_template_all_present`       | all placeholders replaced                 |
| `fill_template_missing_var`       | missing key leaves placeholder unchanged  |
| `fill_template_extra_key`         | extra keys in JSON are silently ignored   |


### `src/handlers/evaluations.rs`


| Test                               | What it checks                                |
| ---------------------------------- | --------------------------------------------- |
| `build_criteria_empty_rubric`      | falls back to generic 4-dimension rubric      |
| `build_criteria_custom_rubric`     | formats name, weight %, description correctly |
| `parse_judge_output_valid`         | all fields parsed correctly                   |
| `parse_judge_output_missing_score` | falls back to dimension mean                  |
| `parse_judge_output_null`          | returns safe defaults, does not panic         |
| `preview_short_string`             | returned unchanged when under limit           |
| `preview_long_string`              | truncated with `...` suffix                   |


---

## 4. Integration Tests

Real database, mocked LLM. Go in a top-level `tests/` folder.

```
tests/
  prompts.rs
  datasets.rs
  evaluations.rs
```

Each test uses `#[sqlx::test(migrations = "migrations")]` which creates a fresh isolated
PostgreSQL database before the test and drops it after — no manual cleanup needed.

### `tests/prompts.rs`


| Test                            | What it checks                                                            |
| ------------------------------- | ------------------------------------------------------------------------- |
| `create_prompt_returns_201`     | persists prompt, returns correct fields                                   |
| `list_prompts_empty_db`         | returns empty array on fresh DB                                           |
| `list_prompts_after_create`     | returns all created prompts                                               |
| `get_prompt_not_found`          | 404 for unknown ID                                                        |
| `get_prompt_after_create`       | 200 with correct body                                                     |
| `update_prompt_partial`         | modifies only provided fields, leaves others unchanged                    |
| `delete_prompt`                 | removes prompt, subsequent get returns 404                                |
| `delete_prompt_not_found`       | 404 when ID does not exist                                                |
| `generate_prompt_import_mode`   | Anthropic call mocked — returns rubric + domain, template stored verbatim |
| `generate_prompt_generate_mode` | Anthropic call mocked — returns template + rubric + variables             |


### `tests/datasets.rs`


| Test                            | What it checks                               |
| ------------------------------- | -------------------------------------------- |
| `create_dataset_returns_201`    | persists dataset with correct fields         |
| `list_datasets_empty_db`        | returns empty array                          |
| `get_dataset_not_found`         | 404 for unknown ID                           |
| `create_dataset_from_questions` | creates dataset + questions in one call      |
| `get_dataset_questions`         | returns questions linked to dataset in order |
| `update_dataset_name`           | name updated, other fields unchanged         |
| `delete_dataset`                | removed, subsequent get returns 404          |


### `tests/evaluations.rs`


| Test                               | What it checks                                    |
| ---------------------------------- | ------------------------------------------------- |
| `run_evaluation_persists_run`      | evaluation run row saved to DB                    |
| `run_evaluation_persists_details`  | one detail row per question                       |
| `run_evaluation_per_prompt_scores` | per-prompt averages computed correctly            |
| `run_evaluation_with_context`      | context client call mocked, KB section injected   |
| `run_evaluation_missing_dataset`   | returns 400                                       |
| `run_evaluation_missing_prompt`    | returns 400                                       |
| `list_evaluations`                 | returns persisted runs ordered by date descending |
| `get_evaluation`                   | returns details joined with question text         |
| `get_evaluation_not_found`         | 404 for unknown ID                                |


---

## 5. Mocking Strategy

### LLM calls (`AnthropicClient`)

`AnthropicClient` uses `reqwest` internally. `wiremock` starts a local HTTP server;
point `ANTHROPIC_BASE_URL` at it for tests. Return a fixed JSON shaped like the
Anthropic API response. The handler code is unaware it is talking to a mock.

```rust
// Example setup in a test
let mock_server = MockServer::start().await;
Mock::given(method("POST"))
    .and(path("/v1/messages"))
    .respond_with(ResponseTemplate::new(200).set_body_json(json!({
        "content": [{ "type": "text", "text": "{\"template\": \"...\", \"domain\": \"code_review\"}" }]
    })))
    .mount(&mock_server)
    .await;
```

### Context client

Same approach — `wiremock` intercepts calls to the context engine URL.
Return a fixed context string to exercise the KB injection path end-to-end.

---

## 6. What NOT to Test


| Area                                  | Reason                                                                    |
| ------------------------------------- | ------------------------------------------------------------------------- |
| LLM output quality                    | That is what the eval system itself is for                                |
| `AnthropicClient::send_text` directly | Thin HTTP wrapper; covered indirectly via `wiremock`                      |
| `stats.rs`                            | Returns hardcoded values; no logic to test until real DB queries added    |
| Frontend API contract                 | Covered by the type system — TypeScript + Rust both compile or they don't |


---

## 7. Running Tests

```bash
# All tests
cargo test

# Single module
cargo test template_parser

# Single integration test file
cargo test --test evaluations

# Single test by name
cargo test build_criteria_empty_rubric
```

`#[sqlx::test]` requires `DATABASE_URL` pointing at a running Postgres instance.
The macro handles creating and dropping per-test databases from that instance automatically.
The same instance used in development is fine — test databases are isolated and cleaned up.

---

## 8. Starting Point

The lowest-friction entry point is the unit tests in `src/handlers/evaluations.rs`
for `build_criteria_text` and `parse_judge_output` — zero external dependencies,
no async, no DB. Write those first to get the test harness running, then layer
in the integration tests once the migration file is in place.