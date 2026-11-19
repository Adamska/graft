use std::ops::Range;

use similar::{capture_diff_slices, diff_ratio, Algorithm, DiffTag};

use super::model::{Line, ParsedFile, Row, RowKind, Segment, Token, TokenClass};
use super::tokens::text_hash;

/// Lines whose tokens overlap less than this are highlighted wholesale.
const INLINE_FLOOR: f32 = 0.3;

/// An ordered selection of lines from one file; gaps are allowed and render as folds.
pub struct Side<'a> {
    pub file: &'a ParsedFile,
    pub lines: Vec<u32>,
}

impl<'a> Side<'a> {
    pub fn range(file: &'a ParsedFile, lines: Range<u32>) -> Self {
        Self { file, lines: lines.collect() }
    }

    pub fn whole(file: &'a ParsedFile) -> Self {
        Self::range(file, 0..file.line_count())
    }

    /// Lines of `outer` that are not covered by any of the `holes`.
    pub fn without(file: &'a ParsedFile, outer: Range<u32>, holes: &[Range<u32>]) -> Self {
        Self { file, lines: outer.filter(|line| !holes.iter().any(|hole| hole.contains(line))).collect() }
    }

    fn keys(&self) -> Vec<u64> {
        self.lines.iter().map(|line| text_hash(self.file.line_text(*line).trim())).collect()
    }

    fn line(&self, at: usize, flags: Option<&[bool]>) -> Line {
        let line = self.lines[at];
        Line { number: line + 1, segments: segments(self.file, line, flags) }
    }
}

/// Myers alignment of two line selections with token-level highlights inside changed
/// lines. Lines compare by trimmed text so re-indentation alone is not a change.
/// With `context`, long unchanged runs collapse into fold rows.
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
        if run.len() > context * 2 + 1 {
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

fn tokens_on(file: &ParsedFile, line: u32) -> &[Token] {
    let range = file.line_range(line);
    let first = file.tokens.partition_point(|token| token.end as usize <= range.start);
    let count = file.tokens[first..].iter().take_while(|token| (token.start as usize) < range.end).count();
    &file.tokens[first..first + count]
}

fn clipped<'a>(file: &'a ParsedFile, token: &Token, range: &Range<usize>) -> &'a str {
    &file.source[(token.start as usize).max(range.start)..(token.end as usize).min(range.end)]
}

/// Per-token change flags for a pair of lines, in token order.
fn inline_flags(old_file: &ParsedFile, old_line: u32, new_file: &ParsedFile, new_line: u32) -> (Vec<bool>, Vec<bool>) {
    let keys = |file: &ParsedFile, line: u32| -> Vec<u64> {
        let range = file.line_range(line);
        tokens_on(file, line).iter().map(|token| text_hash(clipped(file, token, &range))).collect()
    };
    let (old_keys, new_keys) = (keys(old_file, old_line), keys(new_file, new_line));
    let ops = capture_diff_slices(Algorithm::Myers, &old_keys, &new_keys);
    if diff_ratio(&ops, old_keys.len(), new_keys.len()) < INLINE_FLOOR {
        return (vec![true; old_keys.len()], vec![true; new_keys.len()]);
    }

    let (mut old_flags, mut new_flags) = (vec![false; old_keys.len()], vec![false; new_keys.len()]);
    for op in ops {
        let (tag, old_range, new_range) = op.as_tag_tuple();
        if tag != DiffTag::Equal {
            old_flags[old_range].fill(true);
            new_flags[new_range].fill(true);
        }
    }
    (old_flags, new_flags)
}

/// Splits a line into syntax-classed runs, merging whitespace into its neighbors.
fn segments(file: &ParsedFile, line: u32, flags: Option<&[bool]>) -> Vec<Segment> {
    let range = file.line_range(line);
    let mut out: Vec<Segment> = Vec::new();
    let mut at = range.start;
    let mut push = |text: &str, class: TokenClass, changed: bool| match out.last_mut() {
        Some(last) if last.changed == changed && (last.class == class || text.trim().is_empty()) => {
            last.text.push_str(text)
        }
        _ => out.push(Segment { text: text.to_string(), class, changed }),
    };

    let mut previous_changed = false;
    for (index, token) in tokens_on(file, line).iter().enumerate() {
        let changed = flags.is_some_and(|flags| flags.get(index).copied().unwrap_or(false));
        let start = (token.start as usize).max(range.start);
        if start > at {
            push(&file.source[at..start], TokenClass::Plain, previous_changed && changed);
        }
        push(clipped(file, token, &range), token.class, changed);
        at = (token.end as usize).min(range.end);
        previous_changed = changed;
    }
    if at < range.end {
        push(&file.source[at..range.end], TokenClass::Plain, false);
    }
    out
}
