use reqwest::Client;
use serde_json::Value;
use std::collections::HashSet;

/// Client for the LightRAG context engine.
/// Constructed from environment variables — returns None if either var is absent,
/// which gracefully disables context retrieval for the whole system.
#[derive(Clone)]
pub struct ContextClient {
    base_url: String,
    api_key: String,
    http: Client,
}

impl ContextClient {
    /// Returns None (not an error) when CONTEXT_ENGINE_URL or
    /// CONTEXT_ENGINE_API_KEY are missing from the environment.
    pub fn from_env() -> Option<Self> {
        let raw_url = std::env::var("CONTEXT_ENGINE_URL").ok()?;
        let api_key = std::env::var("CONTEXT_ENGINE_API_KEY").ok()?;
        // Strip any query string — params are added dynamically via .query().
        let base_url = raw_url
            .split('?')
            .next()
            .unwrap_or(&raw_url)
            .trim_end_matches('/')
            .to_string();
        println!("🔌 Context engine base URL: {base_url}");
        Some(Self { base_url, api_key, http: Client::new() })
    }

    /// Fetch context for a question from the given project.
    /// Returns a formatted string ready to inject into a prompt.
    /// Returns Err on network/auth failure — callers should log and continue
    /// without context rather than failing the evaluation.
    pub async fn fetch_context(&self, question: &str, project: &str) -> Result<String, String> {
        // Build the request first so we can log the exact URL before sending.
        let request = self
            .http
            .get(&self.base_url)
            .query(&[
                ("project", project),
                ("engine", "lightrag"),
                ("question", question),
                ("search", "all"),
                ("keywords", "true"),
                ("format", "json_string_with_json"),
            ])
            .header("Authorization", format!("Bearer {}", self.api_key))
            .build()
            .map_err(|e| format!("Failed to build context request: {e}"))?;

        println!("🔍 Context API → {}", request.url());

        let response = self
            .http
            .execute(request)
            .await
            .map_err(|e| format!("Context API request failed: {e}"))?;

        let status = response.status();
        println!("🔍 Context API ← HTTP {}", status);

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("Context API HTTP {status}: {body}"));
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| format!("Context API JSON parse error: {e}"))?;

        let context = extract_context(&json);

        println!(
            "🔍 Context extracted — relations: {}  text_units: {}  total chars: {}",
            json.get("relations_context").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
            json.get("text_units_context").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
            context.len()
        );

        Ok(context)
    }
}

/// Extract a compact, prompt-ready context string from the LightRAG response.
/// Uses relation descriptions (sorted by weight) as the primary signal,
/// supplemented by up to 3 document excerpts.
fn extract_context(json: &Value) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Relation facts: graph edges sorted by relevance weight, descriptions split on <SEP>.
    if let Some(relations) = json.get("relations_context").and_then(|v| v.as_array()) {
        let mut sorted: Vec<&Value> = relations.iter().collect();
        sorted.sort_by(|a, b| {
            let wa = a.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let wb = b.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
            wb.partial_cmp(&wa).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut facts: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        for rel in sorted {
            if let Some(desc) = rel.get("description").and_then(|v| v.as_str()) {
                for fact in desc.split("<SEP>") {
                    let fact = fact.trim().to_string();
                    if !fact.is_empty() && seen.insert(fact.clone()) {
                        facts.push(fact);
                    }
                }
            }
        }

        if !facts.is_empty() {
            parts.push(format!(
                "Key facts:\n{}",
                facts
                    .iter()
                    .map(|f| format!("- {f}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    }

    // Document excerpts: top 3 text chunks, capped at 600 chars each.
    // Chunks that are purely document metadata (PDF generator warnings, empty content) are skipped.
    if let Some(units) = json.get("text_units_context").and_then(|v| v.as_array()) {
        let excerpts: Vec<String> = units
            .iter()
            .filter_map(|u| u.get("content").and_then(|v| v.as_str()))
            .filter(|c| {
                let lower = c.to_lowercase();
                // Skip PDF/tool metadata lines that add noise without content value.
                !lower.contains("evaluation warning") &&
                !lower.contains("spire.doc") &&
                !lower.contains("this document was created") &&
                c.trim().len() > 50
            })
            .take(3)
            .map(|c| {
                let s = c.trim();
                let chars: Vec<char> = s.chars().collect();
                if chars.len() > 600 {
                    format!("{}…", chars[..600].iter().collect::<String>())
                } else {
                    s.to_string()
                }
            })
            .collect();

        if !excerpts.is_empty() {
            parts.push(format!("Supporting excerpts:\n{}", excerpts.join("\n\n")));
        }
    }

    parts.join("\n\n")
}
