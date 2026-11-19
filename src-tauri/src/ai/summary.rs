use std::fmt::Write as _;

use crate::ast_diff::blocks::{Block, Location};
use crate::ast_diff::model::{ChangeKind, Line, RowKind};
use crate::ast_diff::render::render;
use crate::ast_diff::Analysis;

const EXCERPT_LINES: usize = 14;
const BUDGET_BYTES: usize = 28_000;

pub fn line_text(line: &Line) -> String {
    line.segments.iter().map(|segment| segment.text.as_str()).collect()
}

/// Text that changed on one side of a block: whole added or deleted lines, and both
/// sides of edited ones.
pub fn changed_lines(block: &Block, new_side: bool) -> Vec<String> {
    let wanted = if new_side { RowKind::Add } else { RowKind::Del };
    let rows = block.rows.iter().filter(|row| row.kind == wanted || row.kind == RowKind::Change);
    rows.filter_map(|row| if new_side { row.new.as_ref() } else { row.old.as_ref() }).map(line_text).collect()
}

fn place(location: &Option<Location>) -> String {
    match location {
        Some(Location { path: Some(path), start, .. }) => format!("{path}:L{start}"),
        Some(location) => format!("L{}", location.start),
        None => String::new(),
    }
}

/// One-line, human-readable description of a block, e.g. `moved and edited (89% similar)
/// method AuthToken::validate, L27 -> L35`.
pub fn describe(block: &Block) -> String {
    let entity = block.entity.map_or("lines".to_string(), |kind| format!("{kind:?}").to_lowercase());
    let subject = if block.qualified.is_empty() {
        place(&block.new.clone().or(block.old.clone()))
    } else {
        block.qualified.clone()
    };
    let moved = format!("{} -> {}", place(&block.old), place(&block.new));
    match block.kind {
        ChangeKind::Moved => format!("moved unchanged {entity} {subject}, {moved}"),
        ChangeKind::MovedEdited => {
            format!("moved and edited ({}% similar) {entity} {subject}, {moved}", block.similarity)
        }
        ChangeKind::Signature => match &block.renamed_from {
            Some(previous) => format!("renamed {entity} {previous} to {subject}"),
            None => format!("changed signature of {entity} {subject}"),
        },
        ChangeKind::Body => format!("changed body of {entity} {subject}"),
        ChangeKind::Modified => format!("changed signature and body of {entity} {subject}"),
        ChangeKind::Added => format!("added {entity} {subject}"),
        ChangeKind::Deleted => format!("deleted {entity} {subject}"),
        ChangeKind::Lines => format!("edited lines at {subject}"),
        ChangeKind::Origin => format!("{entity} {subject} moved away to {}", place(&block.new)),
    }
}

/// Structural summary of a changeset for a language model: one line per declaration-level
/// change followed by a short excerpt of the lines that actually differ. Changes beyond
/// the size budget are counted in a closing note rather than dropped silently.
pub fn for_model(analysis: &Analysis) -> String {
    let mut text = String::new();
    let mut omitted = 0usize;
    for file in 0..analysis.files.len() {
        let diff = render(analysis, file);
        let language = diff.language.unwrap_or("text");
        let _ = writeln!(text, "## {} ({language}, {:?})", diff.path, analysis.files[file].change.status);
        if diff.binary {
            text.push_str("binary file changed\n");
        }
        for block in diff.blocks.iter().filter(|block| block.kind != ChangeKind::Origin || block.link.is_none()) {
            if text.len() > BUDGET_BYTES {
                omitted += 1;
                continue;
            }
            let _ = writeln!(text, "- {}", describe(block));
            let removed = changed_lines(block, false).into_iter().map(|line| format!("    -{line}"));
            let added = changed_lines(block, true).into_iter().map(|line| format!("    +{line}"));
            let excerpt: Vec<String> = removed.chain(added).collect();
            excerpt.iter().take(EXCERPT_LINES).for_each(|line| text.push_str(&format!("{line}\n")));
            if excerpt.len() > EXCERPT_LINES {
                let _ = writeln!(text, "    ... {} more changed lines", excerpt.len() - EXCERPT_LINES);
            }
        }
        text.push('\n');
    }
    if omitted > 0 {
        let _ = writeln!(text, "[{omitted} further changes omitted to keep this summary short]");
    }
    text
}
