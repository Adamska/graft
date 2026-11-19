//! Helpers shared by the structural diff test suites.

use graft_lib::ast_diff::blocks::FileDiff;
use graft_lib::ast_diff::render::render;
use graft_lib::ast_diff::{analyze, Analysis, ChangeKind, SourcePair};
use graft_lib::git::{ChangeStatus, FileChange};

pub fn pair(path: &str, old: &str, new: &str) -> SourcePair {
    SourcePair {
        change: FileChange { path: path.to_string(), old_path: None, status: ChangeStatus::Modified },
        old: Some(old.to_string()),
        new: Some(new.to_string()),
    }
}

/// `(kind, qualified name)` of every change, in the engine's stable order.
pub fn kinds(analysis: &Analysis) -> Vec<(ChangeKind, String)> {
    analysis
        .changes
        .iter()
        .map(|change| {
            let name = match (change.new, change.old) {
                (Some(at), _) => analysis.new_entity(at).qualified.clone(),
                (None, Some(at)) => analysis.old_entity(at).qualified.clone(),
                (None, None) => unreachable!("a change always has a side"),
            };
            (change.kind, name)
        })
        .collect()
}

pub fn diff(path: &str, old: &str, new: &str) -> (Analysis, FileDiff) {
    let analysis = analyze(vec![pair(path, old, new)]);
    let file = render(&analysis, 0);
    (analysis, file)
}
