use std::path::PathBuf;

use graft_lib::ai::settings::{resolve, Provider, StoredSettings};
use graft_lib::ai::{drafting, heuristics, reviewer, summary, Severity};
use graft_lib::changeset::DiffTarget;
use graft_lib::demo::sandbox;
use graft_lib::state::Session;

fn session(name: &str) -> (PathBuf, Session) {
    let root = std::env::temp_dir().join(format!("graft-assistant-{name}-{}", std::process::id()));
    sandbox::generate(&root).expect("generate demo repository");
    let session = Session::open(&root).expect("open demo repository");
    (root, session)
}

#[test]
fn offline_audit_flags_the_regressions_planted_in_the_demo() {
    let (root, session) = session("audit");
    let analysis = session.analyze(&DiffTarget::Unstaged).expect("analyze");

    let findings = heuristics::audit(&analysis);
    let found: Vec<(&str, Severity, &str)> =
        findings.iter().map(|finding| (finding.symbol.as_str(), finding.severity, finding.title.as_str())).collect();

    assert_eq!(
        found,
        [
            ("AuthToken::is_expired", Severity::Medium, "Comparison boundary changed"),
            ("AuthToken::validate", Severity::High, "Error path removed"),
            ("AuthToken::remaining_ttl", Severity::Medium, "New code can abort at runtime"),
        ]
    );
    assert!(findings[0].detail.contains("`>` became `>=`"), "{}", findings[0].detail);
    assert!(findings[1].detail.contains("AuthError::Malformed"), "{}", findings[1].detail);
    assert!(findings.iter().all(|finding| finding.path == "src/auth.rs" && finding.source == "heuristic"));
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn offline_draft_is_a_conventional_commit_built_from_structure() {
    let (root, session) = session("draft");

    let staged = session.analyze(&DiffTarget::Staged).expect("analyze staged");
    assert_eq!(drafting::commit_message(&staged.summary()), "refactor(api): rework request");

    let unstaged = session.analyze(&DiffTarget::Unstaged).expect("analyze unstaged");
    let message = drafting::commit_message(&unstaged.summary());
    let (subject, body) = message.split_once("\n\n").expect("subject and body");
    assert!(subject.len() <= 72, "{subject}");
    assert!(subject.starts_with("refactor: update is_expired, move and update validate"), "{subject}");
    assert!(body.contains("- relocate and update `AuthToken::validate` in src/auth.rs"), "{body}");
    assert!(body.contains("- add `AuthToken::remaining_ttl` to src/auth.rs"), "{body}");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn model_summary_names_every_change_and_quotes_only_changed_lines() {
    let (root, session) = session("summary");
    let analysis = session.analyze(&DiffTarget::Unstaged).expect("analyze");

    let text = summary::for_model(&analysis);

    assert!(text.contains("## src/auth.rs (Rust, Modified)"), "{text}");
    assert!(text.contains("- moved and edited (89% similar) method AuthToken::validate, L27 -> L35"), "{text}");
    assert!(text.contains("    -            Some(deadline) => now > deadline,"), "{text}");
    assert!(text.contains("    +            Some(deadline) => now >= deadline,"), "{text}");
    assert!(!text.contains("pub fn refresh"), "unchanged declarations must not be sent: {text}");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn without_a_provider_the_reviewer_stays_offline_and_reports_why_when_misconfigured() {
    let (root, session) = session("fallback");
    let analysis = session.analyze(&DiffTarget::Unstaged).expect("analyze");

    let offline = reviewer::review(&analysis, Ok(None));
    assert_eq!(offline.findings.len(), 3);
    assert!(offline.model.is_none() && offline.notice.is_none());

    let incomplete = StoredSettings { provider: Some(Provider::Ollama), ..StoredSettings::default() };
    let draft = reviewer::draft_commit(&analysis, resolve(&incomplete).map_err(|reason| reason.to_string()));
    assert!(draft.model.is_none());
    assert!(draft.message.starts_with("refactor"), "{}", draft.message);
    if std::env::var("GRAFT_AI_MODEL").is_err() {
        assert!(draft.notice.as_deref().is_some_and(|notice| notice.contains("no model chosen")), "{:?}", draft.notice);
    }
    std::fs::remove_dir_all(root).ok();
}
