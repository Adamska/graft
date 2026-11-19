use std::ops::Range;

use similar::{capture_diff_slices, Algorithm, DiffTag};

use super::inline::{inline_flags, segments};
use super::model::{Line, ParsedFile, Row, RowKind};
use super::tokens::text_hash;

/// Hiding fewer lines than this costs more attention than it saves.
const MIN_FOLD: usize = 4;

/// An ordered selection of lines from one file; gaps are allowed and render as folds.
pub struct Side<'a> {
    pub file: &'a ParsedFile,
    pub lines: Vec<u32>,
    /// Compare lines byte for byte instead of ignoring leading and trailing whitespace.
    pub exact: bool,
}

impl<'a> Side<'a> {
    pub fn new(file: &'a ParsedFile, lines: Vec<u32>) -> Self {
        Self { file, lines, exact: false }
    }

    pub fn whole(file: &'a ParsedFile) -> Self {
        Self::new(file, (0..file.line_count()).collect())
    }

    /// The whole file compared the way `git diff` would, so re-indentation is a change.
    pub fn exact(file: &'a ParsedFile) -> Self {
        Self { exact: true, ..Self::whole(file) }
    }

    /// Lines of `outer` that are not covered by any of the `holes`.
    pub fn without(file: &'a ParsedFile, outer: Range<u32>, holes: &[Range<u32>]) -> Self {
        Self::new(file, outer.filter(|line| !holes.iter().any(|hole| hole.contains(line))).collect())
    }

    fn keys(&self) -> Vec<u64> {
        let key = |line: &u32| {
            let text = self.file.line_text(*line);
            text_hash(if self.exact { text } else { text.trim() })
        };
        self.lines.iter().map(key).collect()
    }

    fn line(&self, at: usize, flags: Option<&[bool]>) -> Line {
        let line = self.lines[at];
        Line { number: line + 1, segments: segments(self.file, line, flags) }
    }
}

/// Myers alignment of two line selections with token-level highlights inside changed
/// lines. Unless a side asks for exact comparison, lines compare by trimmed text so
/// re-indentation alone is not a change. With `context`, long unchanged runs fold.
pub fn align(old: &Side<'_>, new: &Side<'_>, context: Option<usize>) -> Vec<Row> {
    let ops = capture_diff_slices(Algorithm::Myers, &old.keys(), &new.keys());
    let mut rows = Vec::with_capacity(old.lines.len().max(new.lines.len()));
    let row = |kind, old, new| Row { kind, old, new, hidden: 0 };

    for op in ops {
        let (tag, old_range, new_range) = op.as_tag_tuple();
        match tag {
            DiffTag::Equal => rows.extend(
                old_range
                    .zip(new_range)
                    .map(|(o, n)| row(RowKind::Context, Some(old.line(o, None)), Some(new.line(n, None)))),
            ),
            DiffTag::Delete => rows.extend(old_range.map(|o| row(RowKind::Del, Some(old.line(o, None)), None))),
            DiffTag::Insert => rows.extend(new_range.map(|n| row(RowKind::Add, None, Some(new.line(n, None))))),
            DiffTag::Replace => {
                let paired = old_range.len().min(new_range.len());
                for offset in 0..paired {
                    let (o, n) = (old_range.start + offset, new_range.start + offset);
                    let (old_flags, new_flags) = inline_flags(old.file, old.lines[o], new.file, new.lines[n]);
                    rows.push(row(
                        RowKind::Change,
                        Some(old.line(o, Some(&old_flags))),
                        Some(new.line(n, Some(&new_flags))),
                    ));
                }
                rows.extend(
                    (old_range.start + paired..old_range.end).map(|o| row(RowKind::Del, Some(old.line(o, None)), None)),
                );
                rows.extend(
                    (new_range.start + paired..new_range.end).map(|n| row(RowKind::Add, None, Some(new.line(n, None)))),
                );
            }
        }
    }

    let rows = mark_gaps(rows);
    match context {
        Some(context) => fold_context(rows, context),
        None => rows,
    }
}

pub fn has_changes(rows: &[Row]) -> bool {
    rows.iter().any(|row| !matches!(row.kind, RowKind::Context | RowKind::Fold))
}

/// Inserts a fold wherever consecutive rows skip source lines.
fn mark_gaps(rows: Vec<Row>) -> Vec<Row> {
    let mut out = Vec::with_capacity(rows.len());
    let (mut last_old, mut last_new) = (None, None);
    for row in rows {
        let old_gap = gap(last_old, row.old.as_ref());
        let new_gap = gap(last_new, row.new.as_ref());
        if old_gap.max(new_gap) > 0 {
            out.push(Row::fold(old_gap.max(new_gap)));
        }
        last_old = row.old.as_ref().map(|line| line.number).or(last_old);
        last_new = row.new.as_ref().map(|line| line.number).or(last_new);
        out.push(row);
    }
    out
}

fn gap(last: Option<u32>, line: Option<&Line>) -> u32 {
    match (last, line) {
        (Some(last), Some(line)) => line.number.saturating_sub(last + 1),
        _ => 0,
    }
}

fn fold_context(rows: Vec<Row>, context: usize) -> Vec<Row> {
    let mut out = Vec::with_capacity(rows.len());
    let mut run: Vec<Row> = Vec::new();
    let flush = |run: &mut Vec<Row>, out: &mut Vec<Row>| {
        if run.len() >= context * 2 + MIN_FOLD {
            let hidden = run.len() - context * 2;
            let tail = run.split_off(run.len() - context);
            out.extend(run.drain(..context));
            out.push(Row::fold(hidden as u32));
            out.extend(tail);
            run.clear();
        } else {
            out.append(run);
        }
    };
    for row in rows {
        if row.kind == RowKind::Context {
            run.push(row);
        } else {
            flush(&mut run, &mut out);
            out.push(row);
        }
    }
    flush(&mut run, &mut out);
    out
}
