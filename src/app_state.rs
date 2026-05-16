use axum::extract::FromRef;
use sqlx::PgPool;

use crate::llm::anthropic_client::AnthropicClient;
use crate::llm::context_client::ContextClient;

/// Shared application state injected into every route.
///
/// Axum's `FromRef` impls below let handlers declare exactly what they need:
///   - `State<PgPool>`                  → database only
///   - `State<AnthropicClient>`         → LLM client only
///   - `State<Option<ContextClient>>`   → context engine (None when not configured)
///   - `State<AppState>`                → everything (rarely needed)

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub llm: AnthropicClient,
    pub context: Option<ContextClient>,
}

impl AppState {
    pub fn new(pool: PgPool, llm: AnthropicClient, context: Option<ContextClient>) -> Self {
        Self { pool, llm, context }
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

impl FromRef<AppState> for Option<ContextClient> {
    fn from_ref(state: &AppState) -> Self {
        state.context.clone()
    }
}
