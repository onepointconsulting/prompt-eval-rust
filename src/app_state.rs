use axum::extract::FromRef;
use sqlx::PgPool;

use crate::llm::anthropic_client::AnthropicClient;

/// Shared application state injected into every route.
///
/// Axum's `FromRef` impls below let handlers declare exactly what they need:
///   - `State<PgPool>` → database only
///   - `State<AnthropicClient>` → LLM client only
///   - `State<AppState>` → both (rarely needed)
///
/// Axum calls the appropriate `from_ref` automatically at dispatch time.

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub llm: AnthropicClient,
}

impl AppState {
    pub fn new(pool: PgPool, llm: AnthropicClient) -> Self {
        Self { pool, llm }
    }
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl FromRef<AppState> for AnthropicClient {
    fn from_ref(state: &AppState) -> Self {
        state.llm.clone()
    }
}
