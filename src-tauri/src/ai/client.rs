use std::time::Duration;

use serde_json::{json, Value};

use super::settings::{Endpoint, Provider};

const TIMEOUT: Duration = Duration::from_secs(120);
const ANTHROPIC_VERSION: &str = "2026-10-21";
/// Leaves room for the model's reasoning ahead of a short answer.
const MAX_TOKENS: u32 = 16_000;

/// Sends one system + user exchange and returns the assistant's text.
///
/// Anthropic is called through its Messages API; OpenAI and Ollama share the
/// chat-completions shape, so any compatible server works by changing the base URL.
pub fn complete(endpoint: &Endpoint, system: &str, user: &str) -> Result<String, String> {
    let config = ureq::Agent::config_builder().timeout_global(Some(TIMEOUT)).http_status_as_error(false).build();
    let agent = ureq::Agent::new_with_config(config);
    let base = endpoint.base_url.trim_end_matches('/');

    let request = match endpoint.provider {
        Provider::Anthropic => agent
            .post(format!("{base}/v1/messages"))
            .header("x-api-key", endpoint.api_key.as_deref().unwrap_or_default())
            .header("anthropic-version", ANTHROPIC_VERSION),
        Provider::Openai | Provider::Ollama => {
            let request = agent.post(format!("{base}/chat/completions"));
            match &endpoint.api_key {
                Some(key) => request.header("authorization", format!("Bearer {key}")),
                None => request,
            }
        }
    };
    let body = match endpoint.provider {
        Provider::Anthropic => json!({
            "model": endpoint.model,
            "max_tokens": MAX_TOKENS,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
        }),
        Provider::Openai | Provider::Ollama => json!({
            "model": endpoint.model,
            "messages": [{ "role": "system", "content": system }, { "role": "user", "content": user }],
        }),
    };

    let mut response = request.send_json(&body).map_err(|error| format!("request failed: {error}"))?;
    let status = response.status();
    let reply: Value = response.body_mut().read_json().map_err(|error| format!("unreadable response: {error}"))?;
    if !status.is_success() {
        let message = reply["error"]["message"].as_str().unwrap_or("no error message");
        return Err(format!("{} returned {status}: {message}", endpoint.model));
    }

    let text = match endpoint.provider {
        Provider::Anthropic => {
            if reply["stop_reason"] == "refusal" {
                return Err(format!("{} declined the request", endpoint.model));
            }
            let blocks = reply["content"].as_array().map(Vec::as_slice).unwrap_or_default();
            blocks.iter().filter(|block| block["type"] == "text").filter_map(|block| block["text"].as_str()).collect()
        }
        Provider::Openai | Provider::Ollama => {
            reply["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_string()
        }
    };
    let text = text.trim();
    if text.is_empty() {
        return Err(format!("{} returned an empty reply", endpoint.model));
    }
    Ok(text.to_string())
}
