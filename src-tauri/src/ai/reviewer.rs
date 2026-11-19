use serde::Deserialize;

use super::settings::Endpoint;
use super::{client, drafting, heuristics, summary, CommitDraft, RiskFinding, RiskReport, Severity};
use crate::ast_diff::Analysis;

const COMMIT_SYSTEM: &str = "You write Git commit messages. Follow Conventional Commits 1.0.0: \
`<type>(<scope>): <imperative lowercase description without trailing period>`, with a subject of at most \
72 characters. Add a short body only when the change needs explaining. You receive a structural summary \
of the staged changes, produced by a syntax-tree diff: each line names a declaration that moved, was \
edited, added, or deleted, followed by an excerpt of the lines that differ. Describe what changed and \
why it matters, not the diff mechanics. Reply with the commit message only: no code fences, no commentary.";

const REVIEW_SYSTEM: &str = "You review a code change for regressions. You receive a structural summary \
produced by a syntax-tree diff: each line names a declaration that moved, was edited, added, or deleted, \
followed by an excerpt of the lines that differ. Report only concrete risks you can point to in the \
excerpt: logic regressions, unhandled error paths, and breaking signature changes. Do not report style \
issues or speculate about code you cannot see. Reply with a JSON array and nothing else. Each element has \
the fields `path`, `symbol` (the declaration name exactly as given), `severity` (`high`, `medium`, or \
`low`), `title` (at most eight words), and `detail` (one sentence). Reply with `[]` when nothing is risky.";

#[derive(Deserialize)]
struct ModelFinding {
    path: String,
    symbol: String,
    severity: Severity,
    title: String,
    detail: String,
}

/// Drafts a commit message with the configured model, or from built-in rules when there
/// is none or the request fails.
pub fn draft_commit(analysis: &Analysis, endpoint: Result<Option<Endpoint>, String>) -> CommitDraft {
    let fallback = |notice| CommitDraft { message: drafting::commit_message(&analysis.summary()), model: None, notice };
    match endpoint {
        Ok(None) => fallback(None),
        Err(reason) => fallback(Some(reason)),
        Ok(Some(endpoint)) => match client::complete(&endpoint, COMMIT_SYSTEM, &summary::for_model(analysis)) {
            Ok(message) => CommitDraft { message: strip_fences(&message), model: Some(endpoint.model), notice: None },
            Err(reason) => fallback(Some(reason)),
        },
    }
}

/// Runs the deterministic audit and, when a model is configured, adds its findings.
pub fn review(analysis: &Analysis, endpoint: Result<Option<Endpoint>, String>) -> RiskReport {
    let mut report = RiskReport { findings: heuristics::audit(analysis), model: None, notice: None };
    let endpoint = match endpoint {
        Ok(Some(endpoint)) => endpoint,
        Ok(None) => return report,
        Err(reason) => return RiskReport { notice: Some(reason), ..report },
    };

    let reply = client::complete(&endpoint, REVIEW_SYSTEM, &summary::for_model(analysis));
    match reply.and_then(|text| parse_findings(&text)) {
        Ok(found) => {
            for finding in found {
                let duplicate = report.findings.iter().any(|known| {
                    known.path == finding.path && known.symbol == finding.symbol && known.title == finding.title
                });
                if !duplicate {
                    report.findings.push(RiskFinding {
                        path: finding.path,
                        symbol: finding.symbol,
                        severity: finding.severity,
                        title: finding.title,
                        detail: finding.detail,
                        source: endpoint.model.clone(),
                    });
                }
            }
            report.model = Some(endpoint.model);
        }
        Err(reason) => report.notice = Some(reason),
    }
    report
}

/// Accepts the JSON array even when the model wraps it in prose or a code fence.
fn parse_findings(text: &str) -> Result<Vec<ModelFinding>, String> {
    let (start, end) = (text.find('['), text.rfind(']'));
    let json = match (start, end) {
        (Some(start), Some(end)) if end > start => &text[start..=end],
        _ => return Err("the model did not return a JSON array of findings".to_string()),
    };
    serde_json::from_str(json).map_err(|error| format!("the model returned malformed findings: {error}"))
}

fn strip_fences(message: &str) -> String {
    let trimmed = message.trim();
    let inner = trimmed
        .strip_prefix("```")
        .and_then(|rest| rest.strip_suffix("```"))
        .map(|rest| rest.trim_start_matches(|c: char| c.is_alphanumeric()));
    inner.unwrap_or(trimmed).trim().to_string()
}
