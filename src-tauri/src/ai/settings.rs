use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;

const FILE_NAME: &str = "assistant.json";
const ANTHROPIC_MODEL: &str = "claude-opus-5-5";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Anthropic,
    Openai,
    Ollama,
}

impl Provider {
    fn default_base_url(self) -> &'static str {
        match self {
            Self::Anthropic => "https://api.anthropic.com",
            Self::Openai => "https://api.openai.com/v1",
            Self::Ollama => "http://localhost:11434/v1",
        }
    }

    fn key_variable(self) -> Option<&'static str> {
        match self {
            Self::Anthropic => Some("ANTHROPIC_API_KEY"),
            Self::Openai => Some("OPENAI_API_KEY"),
            Self::Ollama => None,
        }
    }
}

/// What the user saved in the app; every field falls back to the environment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StoredSettings {
    pub provider: Option<Provider>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
}

/// A fully resolved endpoint, ready to send requests to.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub provider: Provider,
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

/// Safe-to-display view of the configuration; never carries the key itself.
#[derive(Debug, Clone, Serialize)]
pub struct Status {
    pub provider: Option<Provider>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub has_key: bool,
    pub ready: bool,
    pub detail: String,
}

pub fn load(config_dir: &Path) -> StoredSettings {
    let stored = std::fs::read(config_dir.join(FILE_NAME)).ok();
    stored.and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
}

pub fn save(config_dir: &Path, settings: &StoredSettings) -> Result<()> {
    std::fs::create_dir_all(config_dir)?;
    let path = config_dir.join(FILE_NAME);
    std::fs::write(&path, serde_json::to_vec_pretty(settings).map_err(std::io::Error::other)?)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

fn variable(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

fn filled(value: &Option<String>) -> Option<String> {
    value.as_ref().map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

/// Resolves saved settings over `GRAFT_AI_*` variables over provider conventions.
/// `Err` explains what is missing; `Ok(None)` means no provider was chosen at all.
pub fn resolve(stored: &StoredSettings) -> std::result::Result<Option<Endpoint>, String> {
    let named = variable("GRAFT_AI_PROVIDER").and_then(|name| match name.to_ascii_lowercase().as_str() {
        "anthropic" => Some(Provider::Anthropic),
        "openai" => Some(Provider::Openai),
        "ollama" => Some(Provider::Ollama),
        _ => None,
    });
    let inferred = [Provider::Anthropic, Provider::Openai]
        .into_iter()
        .find(|provider| provider.key_variable().and_then(variable).is_some());
    let Some(provider) = stored.provider.or(named).or(inferred) else {
        return Ok(None);
    };

    let api_key = filled(&stored.api_key)
        .or_else(|| variable("GRAFT_AI_API_KEY"))
        .or_else(|| provider.key_variable().and_then(variable));
    if api_key.is_none() && provider != Provider::Ollama {
        return Err(format!("no API key for {provider:?}; set GRAFT_AI_API_KEY or save one in settings"));
    }
    let default_model = (provider == Provider::Anthropic).then(|| ANTHROPIC_MODEL.to_string());
    let Some(model) = filled(&stored.model).or_else(|| variable("GRAFT_AI_MODEL")).or(default_model) else {
        return Err(format!("no model chosen for {provider:?}; set GRAFT_AI_MODEL or save one in settings"));
    };
    let base_url = filled(&stored.base_url)
        .or_else(|| variable("GRAFT_AI_BASE_URL"))
        .unwrap_or_else(|| provider.default_base_url().to_string());
    Ok(Some(Endpoint { provider, base_url, model, api_key }))
}

pub fn status(stored: &StoredSettings) -> Status {
    match resolve(stored) {
        Ok(Some(endpoint)) => Status {
            provider: Some(endpoint.provider),
            base_url: Some(endpoint.base_url),
            has_key: endpoint.api_key.is_some(),
            detail: format!("Using {}", endpoint.model),
            model: Some(endpoint.model),
            ready: true,
        },
        Ok(None) => Status {
            provider: None,
            base_url: None,
            model: None,
            has_key: false,
            ready: false,
            detail: "No model configured; built-in heuristics are used".to_string(),
        },
        Err(detail) => Status {
            provider: stored.provider,
            base_url: filled(&stored.base_url),
            model: filled(&stored.model),
            has_key: filled(&stored.api_key).is_some(),
            ready: false,
            detail,
        },
    }
}
