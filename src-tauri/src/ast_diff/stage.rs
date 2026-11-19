use std::collections::HashSet;
use std::ops::Range;

use serde::Deserialize;

use super::model::{ChangeKind, ParsedFile};
use super::{Analysis, Change, EntityRef};

/// Identifies one declaration-level change by name and the lines it starts on.
/// A side is `None` when the declaration does not exist there (or lives in another file).
#[derive(Debug, Clone, Deserialize)]
pub struct EntitySelector {
    pub qualified: String,
    pub old_start: Option<u32>,
    pub new_start: Option<u32>,
}

/// Rewrites the old side of a single-file analysis so that the selected declaration
/// matches the new side while everything else stays untouched. Staging applies this with
/// the index as the old side and the working tree as the new one; unstaging swaps in `HEAD`.
pub fn transplant(analysis: &Analysis, selector: &EntitySelector) -> Option<String> {
    let pair = analysis.files.first()?;
    let (base, donor) = (&pair.old, &pair.new);
    let change = analysis.changes.iter().find(|change| {
        let old = change.old.map(|at| &base.entities[at.index]);
        let new = change.new.map(|at| &donor.entities[at.index]);
        old.map(|entity| entity.lines.start + 1) == selector.old_start
            && new.map(|entity| entity.lines.start + 1) == selector.new_start
            && new.or(old).is_some_and(|entity| entity.qualified == selector.qualified)
    })?;

    let mut edits: Vec<(Range<usize>, String)> = Vec::new();
    let relocated = matches!(change.kind, ChangeKind::Moved | ChangeKind::MovedEdited);
    match (change.old, change.new) {
        (Some(old), Some(new)) if !relocated => {
            edits.push((
                base.entities[old.index].span.clone(),
                donor.source[donor.entities[new.index].span.clone()].to_string(),
            ));
        }
        (old, new) => {
            if let Some(old) = old {
                edits.push((with_blank_line(base, &base.entities[old.index].span), String::new()));
            }
            if let Some(new) = new {
                edits.push(insertion(analysis, base, donor, new.index));
            }
        }
    }

    edits.sort_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut text = base.source.clone();
    for (range, replacement) in edits {
        text.replace_range(range, &replacement);
    }
    Some(text)
}

/// Whether a change can be transplanted on its own. In-place edits to a container are
/// excluded: its span also holds member edits that the user may not want to stage yet.
pub fn is_stageable(analysis: &Analysis, change: &Change) -> bool {
    let container = match (change.new, change.old) {
        (Some(at), _) => analysis.new_entity(at).is_container,
        (None, Some(at)) => analysis.old_entity(at).is_container,
        (None, None) => return false,
    };
    !(container && matches!(change.kind, ChangeKind::Signature | ChangeKind::Body | ChangeKind::Modified))
}

/// Picks where a declaration missing from `base` belongs: after the nearest preceding
/// sibling that both sides share in the same order, else before the next such sibling,
/// else at the end of its container, else at the end of the file.
fn insertion(analysis: &Analysis, base: &ParsedFile, donor: &ParsedFile, index: usize) -> (Range<usize>, String) {
    let entity = &donor.entities[index];
    let mut text = donor.source[entity.span.clone()].to_string();
    if !text.ends_with('\n') {
        text.push('\n');
    }

    let moved: HashSet<usize> = analysis
        .changes
        .iter()
        .filter(|change| matches!(change.kind, ChangeKind::Moved | ChangeKind::MovedEdited))
        .filter_map(|change| change.new.map(|at| at.index))
        .collect();
    let anchor = |sibling: &usize| {
        let origin =
            analysis.origins.get(&EntityRef { file: 0, index: *sibling }).filter(|_| !moved.contains(sibling))?;
        Some(&base.entities[origin.index].span)
    };
    let siblings: Vec<usize> = donor.children_of(entity.parent).collect();
    let position = siblings.iter().position(|sibling| *sibling == index).unwrap_or(0);

    if let Some(span) = siblings[..position].iter().rev().find_map(anchor) {
        return (span.end..span.end, format!("\n{text}"));
    }
    if let Some(span) = siblings[position + 1..].iter().find_map(anchor) {
        return (span.start..span.start, format!("{text}\n"));
    }
    if let Some(parent) = entity.parent.and_then(|parent| analysis.origins.get(&EntityRef { file: 0, index: parent })) {
        let span = &base.entities[parent.index].span;
        let last_line = base.line_starts[base.line_of(span.end.saturating_sub(1)) as usize] as usize;
        let closes = base.source[last_line..span.end].trim_start().starts_with(['}', ')', ']']);
        let at = if closes { last_line } else { span.end };
        return (at..at, format!("\n{text}"));
    }
    let separator = match base.source.as_str() {
        "" => "",
        source if source.ends_with('\n') => "\n",
        _ => "\n\n",
    };
    (base.source.len()..base.source.len(), format!("{separator}{text}"))
}

/// Widens a removal by one neighboring blank line so no double gap is left behind.
fn with_blank_line(file: &ParsedFile, span: &Range<usize>) -> Range<usize> {
    let is_blank = |line: u32| line < file.line_count() && file.line_text(line).trim().is_empty();
    let first = file.line_of(span.start);
    let after = file.line_of(span.end.saturating_sub(1)) + 1;
    if first > 0 && is_blank(first - 1) {
        file.line_starts[first as usize - 1] as usize..span.end
    } else if is_blank(after) {
        let end = file.line_starts.get(after as usize + 1).map_or(file.source.len(), |start| *start as usize);
        span.start..end
    } else {
        span.clone()
    }
}
