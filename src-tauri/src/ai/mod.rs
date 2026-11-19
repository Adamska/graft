//! Optional language-model assistance with a deterministic offline fallback.

pub mod client;
pub mod drafting;
pub mod heuristics;
pub mod reviewer;
pub mod settings;
pub mod summary;

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    High,
    Medium,
    Low,
}

/// A risk attached to one declaration in the diff.
#[derive(Debug, Clone, Serialize)]
pub struct RiskFinding {
    pub path: String,
    pub symbol: String,
    pub severity: Severity,
    pub title: String,
    pub detail: String,
    /// `heuristic`, or the model that reported the finding.
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskReport {
    pub findings: Vec<RiskFinding>,
    /// Model consulted in addition to the built-in rules, if any.
    pub model: Option<String>,
    /// Why the model was not used, when one is configured but the request failed.
    pub notice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitDraft {
    pub message: String,
    pub model: Option<String>,
    pub notice: Option<String>,
}
