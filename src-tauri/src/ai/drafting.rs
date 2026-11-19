use crate::ast_diff::{ChangeKind, ChangeSummary, ChangesetSummary, FileSummary};

const SUBJECT_LIMIT: usize = 72;
const BODY_LIMIT: usize = 12;

/// Drafts a Conventional Commit message from the structural summary alone.
pub fn commit_message(summary: &ChangesetSummary) -> String {
    let changes: Vec<(&FileSummary, &ChangeSummary)> = summary
        .files
        .iter()
        .flat_map(|file| file.changes.iter().map(move |change| (file, change)))
        .filter(|(_, change)| change.kind != ChangeKind::Origin)
        .collect();

    let mut phrases: Vec<String> = changes.iter().map(|(_, change)| phrase(change)).collect();
    phrases.extend(
        summary
            .files
            .iter()
            .filter(|file| file.changes.is_empty())
            .map(|file| format!("update {}", base_name(&file.change.path))),
    );
    phrases.dedup();
    if phrases.is_empty() {
        return String::new();
    }

    let prefix = match scope(summary) {
        Some(scope) => format!("{}({scope}): ", kind(summary, &changes)),
        None => format!("{}: ", kind(summary, &changes)),
    };
    let mut subject = format!("{prefix}{}", phrases[0]);
    let mut used = 1;
    while used < phrases.len() {
        let remaining = phrases.len() - used - 1;
        let joiner = if remaining == 0 { " and " } else { ", " };
        let reserve = if remaining > 0 { " and 00 more".len() } else { 0 };
        if subject.len() + joiner.len() + phrases[used].len() + reserve > SUBJECT_LIMIT {
            break;
        }
        subject = format!("{subject}{joiner}{}", phrases[used]);
        used += 1;
    }
    if used < phrases.len() {
        subject = format!("{subject} and {} more", phrases.len() - used);
    }
    if phrases.len() == 1 {
        return subject;
    }

    let mut body: Vec<String> = changes.iter().take(BODY_LIMIT).map(|(file, change)| bullet(file, change)).collect();
    if changes.len() > BODY_LIMIT {
        body.push(format!("- and {} more changes", changes.len() - BODY_LIMIT));
    }
    if body.is_empty() {
        subject
    } else {
        format!("{subject}\n\n{}", body.join("\n"))
    }
}

fn short(name: &str) -> &str {
    name.rsplit(['.', ':']).next().unwrap_or(name)
}

fn base_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn phrase(change: &ChangeSummary) -> String {
    let name = short(&change.name);
    match change.kind {
        ChangeKind::Added => format!("add {name}"),
        ChangeKind::Deleted => format!("remove {name}"),
        ChangeKind::Moved => format!("move {name}"),
        ChangeKind::MovedEdited => format!("move and update {name}"),
        ChangeKind::Signature => format!("change {name} signature"),
        ChangeKind::Body => format!("update {name}"),
        _ => format!("rework {name}"),
    }
}

fn bullet(file: &FileSummary, change: &ChangeSummary) -> String {
    let path = &file.change.path;
    match (change.kind, &change.other_path) {
        (ChangeKind::Moved, Some(source)) => format!("- move `{}` from {source} to {path}", change.name),
        (ChangeKind::MovedEdited, Some(source)) => {
            format!("- move `{}` from {source} to {path} and update it", change.name)
        }
        (ChangeKind::Moved, None) => format!("- reorder `{}` within {path}", change.name),
        (ChangeKind::MovedEdited, None) => format!("- relocate and update `{}` in {path}", change.name),
        (ChangeKind::Added, _) => format!("- add `{}` to {path}", change.name),
        (ChangeKind::Deleted, _) => format!("- remove `{}` from {path}", change.name),
        (ChangeKind::Signature, _) => format!("- change the signature of `{}` in {path}", change.name),
        _ => format!("- update `{}` in {path}", change.name),
    }
}

/// Picks the commit type from what dominates the changeset.
fn kind(summary: &ChangesetSummary, changes: &[(&FileSummary, &ChangeSummary)]) -> &'static str {
    let every = |test: fn(&str) -> bool| summary.files.iter().all(|file| test(&file.change.path));
    if every(|path| path.ends_with(".md") || path.starts_with("docs/")) {
        return "docs";
    }
    if every(|path| path.contains("test") || path.contains("spec")) {
        return "test";
    }
    let count = |kinds: &[ChangeKind]| changes.iter().filter(|(_, change)| kinds.contains(&change.kind)).count();
    let added = count(&[ChangeKind::Added]);
    let moved = count(&[ChangeKind::Moved, ChangeKind::MovedEdited]);
    let reshaped = count(&[ChangeKind::Signature, ChangeKind::Modified, ChangeKind::Deleted]);
    let edited = count(&[ChangeKind::Body]);
    if changes.is_empty() {
        "chore"
    } else if added > moved + reshaped + edited {
        "feat"
    } else if moved + reshaped >= edited.max(1) {
        "refactor"
    } else {
        "fix"
    }
}

/// A single file names the scope by its stem; otherwise a shared top-level directory does.
fn scope(summary: &ChangesetSummary) -> Option<String> {
    let paths: Vec<&str> = summary.files.iter().map(|file| file.change.path.as_str()).collect();
    if let [only] = paths.as_slice() {
        return base_name(only).split('.').next().map(str::to_string).filter(|stem| !stem.is_empty());
    }
    let first = paths.first()?.split_once('/')?.0;
    paths.iter().all(|path| path.split_once('/').is_some_and(|(top, _)| top == first)).then(|| first.to_string())
}
