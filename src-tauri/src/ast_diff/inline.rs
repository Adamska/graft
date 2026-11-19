//! Token-level detail inside a line: syntax-classed segments and intra-line change marks.

use std::ops::Range;

use similar::{capture_diff_slices, diff_ratio, Algorithm, DiffTag};

use super::model::{ParsedFile, Segment, Token, TokenClass};
use super::tokens::text_hash;

/// Lines whose tokens overlap less than this are highlighted wholesale.
const INLINE_FLOOR: f32 = 0.3;

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
pub(super) fn inline_flags(
    old_file: &ParsedFile,
    old_line: u32,
    new_file: &ParsedFile,
    new_line: u32,
) -> (Vec<bool>, Vec<bool>) {
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
pub(super) fn segments(file: &ParsedFile, line: u32, flags: Option<&[bool]>) -> Vec<Segment> {
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
