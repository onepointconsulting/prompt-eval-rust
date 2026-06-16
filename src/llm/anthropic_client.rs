use axum::http::StatusCode;
use reqwest::Client;
use serde_json::{json, Value};
use std::env;

const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";

#[derive(Clone)]
pub struct AnthropicClient {
    http: Client,
    api_key: String,
    model_haiku: String,
    model_sonnet: String,
}

impl AnthropicClient {
    pub fn from_env() -> Result<Self, StatusCode> {
        let api_key =
            env::var("ANTHROPIC_API_KEY").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let model_haiku =
            env::var("ANTHROPIC_MODEL_HAIKU").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let model_sonnet =
            env::var("ANTHROPIC_MODEL_SONNET").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Self {
            http: Client::new(),
            api_key,
            model_haiku,
            model_sonnet,
        })
    }

    pub fn model_haiku(&self) -> &str {
        &self.model_haiku
    }

    pub fn model_sonnet(&self) -> &str {
        &self.model_sonnet
    }

    pub async fn send_json(
        &self,
        model: &str,
        max_tokens: i32,
        user_prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<Value, StatusCode> {
        let mut request_body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "messages": [{
                "role": "user",
                "content": user_prompt
            }]
        });

        if let Some(system) = system_prompt {
            request_body["system"] = json!(system);
        }

        self.post_body(&request_body).await
    }

    pub async fn send_text(
        &self,
        model: &str,
        max_tokens: i32,
        user_prompt: &str,
        system_prompt: Option<&str>,
    ) -> Result<String, StatusCode> {
        let data = self
            .send_json(model, max_tokens, user_prompt, system_prompt)
            .await?;
        data["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(StatusCode::BAD_GATEWAY)
    }

    /// Send a request whose response is constrained to match `schema`.
    /// The API guarantees the returned text is valid JSON for that schema,
    /// so callers can deserialize it without fallback handling.
    pub async fn send_structured(
        &self,
        model: &str,
        max_tokens: i32,
        user_prompt: &str,
        system_prompt: Option<&str>,
        schema: Value,
    ) -> Result<String, StatusCode> {
        let mut request_body = json!({
            "model": model,
            "max_tokens": max_tokens,
            "output_config": {
                "format": {
                    "type": "json_schema",
                    "schema": schema
                }
            },
            "messages": [{ "role": "user", "content": user_prompt }]
        });

        if let Some(system) = system_prompt {
            request_body["system"] = json!(system);
        }

        let data = self.post_body(&request_body).await?;
        data["content"][0]["text"]
            .as_str()
            .map(|s| s.to_string())
            .ok_or(StatusCode::BAD_GATEWAY)
    }

    /// Shared HTTP layer: POST a request body to the Messages API and
    /// return the parsed response JSON.
    async fn post_body(&self, body: &Value) -> Result<Value, StatusCode> {
        let response = self
            .http
            .post(ANTHROPIC_MESSAGES_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(body)
            .send()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        if !status.is_success() {
            eprintln!("anthropic non-2xx status={} body={}", status, body_text);
            return Err(StatusCode::BAD_GATEWAY);
        }
        serde_json::from_str::<Value>(&body_text).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}
