use std::collections::HashSet;

use serde::Deserialize;

use super::lines::{align, Side};
use super::model::{ParsedFile, RowKind};
use super::FilePair;

/// Changed lines picked in the raw diff, by the one-based numbers shown in its gutters:
/// `old` holds removed lines, `new` holds added ones.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LineSelection {
    #[serde(default)]
    pub old: Vec<u32>,
    #[serde(default)]
    pub new: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// The old side plus the selected changes: staging into the index.
    Apply,
    /// The new side minus the selected changes: unstaging, or discarding from disk.
    Revert,
}

/// Rebuilds a file with only part of a diff applied. The alignment is the one the raw
/// view renders, so a selection always refers to the rows the user saw. Returns `None`
/// when a selected line is not a change in that alignment, which means the file moved on.
pub fn splice(pair: &FilePair, selection: &LineSelection, direction: Direction) -> Option<String> {
    let rows = align(&Side::exact(&pair.old), &Side::exact(&pair.new), None);
    let (old_picks, new_picks): (HashSet<u32>, HashSet<u32>) =
        (selection.old.iter().copied().collect(), selection.new.iter().copied().collect());
    let (mut old_seen, mut new_seen) = (0, 0);

    let mut text = String::with_capacity(pair.old.source.len().max(pair.new.source.len()));
    // Within one run of changes the surviving removals come first, then the additions,
    // matching the order of a unified diff.
    let mut additions: Vec<&str> = Vec::new();
    for row in rows.iter().filter(|row| row.kind != RowKind::Fold) {
        let old = row.old.as_ref().map(|line| line.number);
        let new = row.new.as_ref().map(|line| line.number);
        if row.kind == RowKind::Context {
            additions.drain(..).for_each(|line| push_line(&mut text, line));
            let kept = match direction {
                Direction::Apply => old.map(|number| raw_line(&pair.old, number)),
                Direction::Revert => new.map(|number| raw_line(&pair.new, number)),
            };
            push_line(&mut text, kept.unwrap_or_default());
            continue;
        }
        if let Some(number) = old {
            let picked = old_picks.contains(&number);
            old_seen += usize::from(picked);
            if picked != (direction == Direction::Apply) {
                push_line(&mut text, raw_line(&pair.old, number));
            }
        }
        if let Some(number) = new {
            let picked = new_picks.contains(&number);
            new_seen += usize::from(picked);
            if picked == (direction == Direction::Apply) {
                additions.push(raw_line(&pair.new, number));
            }
        }
    }
    additions.drain(..).for_each(|line| push_line(&mut text, line));
    (old_seen == old_picks.len() && new_seen == new_picks.len()).then_some(text)
}

/// A line with its terminator, so untouched line endings survive byte for byte.
fn raw_line(file: &ParsedFile, number: u32) -> &str {
    let at = number as usize - 1;
    let start = file.line_starts[at] as usize;
    let end = file.line_starts.get(at + 1).map_or(file.source.len(), |next| *next as usize);
    &file.source[start..end]
}

fn push_line(text: &mut String, line: &str) {
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(line);
}
