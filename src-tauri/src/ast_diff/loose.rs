use similar::{capture_diff_slices, Algorithm, DiffTag};

use super::blocks::{Block, Location};
use super::lines::{align, has_changes, Side};
use super::model::{ChangeKind, ParsedFile, Row, RowKind};
use super::render::{capped, Draft, CONTEXT};
use super::tokens::text_hash;
use super::FilePair;

/// Hunks in code that belongs to no declaration: imports, constants, top-level statements.
pub fn drafts(pair: &FilePair, line_map: &[u32], out: &mut Vec<Draft>) {
    let has_entities = !pair.old.entities.is_empty() || !pair.new.entities.is_empty();
    let rows = align(&outside(&pair.old), &outside(&pair.new), Some(CONTEXT));

    for hunk in rows.split(|row| row.kind == RowKind::Fold) {
        let blank_only = hunk.iter().filter(|row| row.kind != RowKind::Context).all(|row| {
            row.old
                .iter()
                .chain(&row.new)
                .all(|line| line.segments.iter().all(|segment| segment.text.trim().is_empty()))
        });
        if !has_changes(hunk) || (has_entities && blank_only) {
            continue;
        }
        let span = |pick: fn(&Row) -> Option<u32>| {
            let mut numbers = hunk.iter().filter_map(pick);
            let start = numbers.next()?;
            Some(Location { path: None, start, end: numbers.next_back().unwrap_or(start) })
        };
        let (old, new) = (
            span(|row| row.old.as_ref().map(|line| line.number)),
            span(|row| row.new.as_ref().map(|line| line.number)),
        );
        let anchor = match (&new, &old) {
            (Some(new), _) => (new.start - 1, 1, 0),
            (None, Some(old)) => (line_map[old.start as usize - 1], 0, old.start),
            (None, None) => continue,
        };
        let block = Block {
            id: 0,
            kind: ChangeKind::Lines,
            entity: None,
            name: String::new(),
            qualified: String::new(),
            old,
            new,
            similarity: 0,
            renamed_from: None,
            link: None,
            stageable: false,
            rows: capped(hunk.to_vec()),
        };
        out.push(Draft { anchor, change: None, block });
    }
}

fn outside(file: &ParsedFile) -> Side<'_> {
    let top: Vec<_> = file.children_of(None).map(|index| file.entities[index].lines.clone()).collect();
    Side::without(file, 0..file.line_count(), &top)
}

/// Maps each old line to the new-file line it aligns with, or would be inserted before.
pub fn line_map(pair: &FilePair) -> Vec<u32> {
    let keys = |file: &ParsedFile| {
        (0..file.line_count()).map(|line| text_hash(file.line_text(line).trim())).collect::<Vec<_>>()
    };
    let mut map = vec![pair.new.line_count(); pair.old.line_count() as usize + 1];
    for op in capture_diff_slices(Algorithm::Myers, &keys(&pair.old), &keys(&pair.new)) {
        let (tag, old_range, new_range) = op.as_tag_tuple();
        for (offset, line) in old_range.enumerate() {
            map[line] = (new_range.start + if tag == DiffTag::Equal { offset } else { 0 }) as u32;
        }
    }
    map
}
