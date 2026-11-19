use std::collections::HashMap;

use super::blocks::{Block, FileDiff, Location};
use super::lines::{align, Side};
use super::loose;
use super::model::{ChangeKind, Entity, ParsedFile, Row};
use super::stage;
use super::{Analysis, Change, EntityRef, FilePair};

pub(super) const CONTEXT: usize = 3;
const MAX_ROWS: usize = 4000;

pub(super) struct Draft {
    /// New-file line the block sorts at, then old-only blocks first, then old line.
    pub anchor: (u32, u8, u32),
    pub change: Option<usize>,
    pub block: Block,
}

/// Lays out one file's changes as blocks ordered by their position in the new file.
/// Deleted declarations and move origins are slotted in where they used to sit.
pub fn render(analysis: &Analysis, file: usize) -> FileDiff {
    let pair = &analysis.files[file];
    let mut drafts = Vec::new();
    if !pair.binary {
        let line_map = loose::line_map(pair);
        for (index, change) in analysis.changes.iter().enumerate() {
            entity_drafts(analysis, file, index, change, &line_map, &mut drafts);
        }
        loose::drafts(pair, &line_map, &mut drafts);
    }
    drafts.sort_by_key(|draft| draft.anchor);

    let mut ids: HashMap<(usize, bool), u32> = HashMap::new();
    for (id, draft) in drafts.iter_mut().enumerate() {
        draft.block.id = id as u32;
        if let Some(change) = draft.change {
            ids.insert((change, draft.block.kind == ChangeKind::Origin), id as u32);
        }
    }
    for draft in &mut drafts {
        if let Some(change) = draft.change {
            draft.block.link = ids.get(&(change, draft.block.kind != ChangeKind::Origin)).copied();
        }
    }

    FileDiff {
        path: pair.change.path.clone(),
        old_path: pair.change.old_path.clone(),
        language: pair.new.language.or(pair.old.language),
        structural: pair.is_structural(),
        binary: pair.binary,
        blocks: drafts.into_iter().map(|draft| draft.block).collect(),
    }
}

/// Plain Myers line diff of the whole file with three lines of context.
pub fn raw_rows(pair: &FilePair) -> Vec<Row> {
    capped(align(&Side::whole(&pair.old), &Side::whole(&pair.new), Some(CONTEXT)))
}

fn entity_drafts(
    analysis: &Analysis,
    file: usize,
    index: usize,
    change: &Change,
    line_map: &[u32],
    out: &mut Vec<Draft>,
) {
    let old = change.old.map(|at| (at, analysis.old_entity(at)));
    let new = change.new.map(|at| (at, analysis.new_entity(at)));
    let locate = |(at, entity): (EntityRef, &Entity)| Location {
        path: (at.file != file).then(|| analysis.files[at.file].change.path.clone()),
        start: entity.lines.start + 1,
        end: entity.lines.end,
    };
    let block = |kind, entity: &Entity, rows| Block {
        id: 0,
        kind,
        entity: Some(entity.kind),
        name: entity.name.clone(),
        qualified: entity.qualified.clone(),
        old: old.map(locate),
        new: new.map(locate),
        similarity: (change.similarity * 100.0).round() as u8,
        renamed_from: old
            .zip(new)
            .filter(|((_, old), (_, new))| old.name != new.name)
            .map(|((_, old), _)| old.name.clone()),
        link: None,
        stageable: stage::is_stageable(analysis, change),
        rows,
    };

    if let Some((_, entity)) = new.filter(|(at, _)| at.file == file) {
        let rows = change_rows(analysis, change);
        out.push(Draft {
            anchor: (entity.lines.start, 1, 0),
            change: Some(index),
            block: block(change.kind, entity, rows),
        });
    }
    if let Some((_, entity)) = old.filter(|(at, _)| at.file == file) {
        let anchor = (line_map[entity.lines.start as usize], 0, entity.lines.start);
        match change.kind {
            ChangeKind::Deleted => {
                let rows = change_rows(analysis, change);
                out.push(Draft { anchor, change: Some(index), block: block(ChangeKind::Deleted, entity, rows) });
            }
            ChangeKind::Moved | ChangeKind::MovedEdited => {
                out.push(Draft { anchor, change: Some(index), block: block(ChangeKind::Origin, entity, Vec::new()) });
            }
            _ => {}
        }
    }
}

fn change_rows(analysis: &Analysis, change: &Change) -> Vec<Row> {
    let whole = matches!(change.kind, ChangeKind::Added | ChangeKind::Deleted | ChangeKind::Moved);
    let side = |file: &'_ ParsedFile, index: Option<usize>| -> Vec<u32> {
        let Some(index) = index else { return Vec::new() };
        let entity = &file.entities[index];
        if whole || !entity.is_container {
            return entity.lines.clone().collect();
        }
        let nested: Vec<_> = file.children_of(Some(index)).map(|child| file.entities[child].lines.clone()).collect();
        Side::without(file, entity.lines.clone(), &nested).lines
    };

    let anchor = change.new.or(change.old).map_or(0, |at| at.file);
    let old_file = change.old.map_or(&analysis.files[anchor].old, |at| &analysis.files[at.file].old);
    let new_file = change.new.map_or(&analysis.files[anchor].new, |at| &analysis.files[at.file].new);
    let old = Side { file: old_file, lines: side(old_file, change.old.map(|at| at.index)) };
    let new = Side { file: new_file, lines: side(new_file, change.new.map(|at| at.index)) };
    let context = (!matches!(change.kind, ChangeKind::Added | ChangeKind::Deleted)).then_some(CONTEXT);
    capped(align(&old, &new, context))
}

pub(super) fn capped(mut rows: Vec<Row>) -> Vec<Row> {
    if rows.len() > MAX_ROWS {
        let hidden = rows.len() - MAX_ROWS;
        rows.truncate(MAX_ROWS);
        rows.push(Row::fold(hidden as u32));
    }
    rows
}
