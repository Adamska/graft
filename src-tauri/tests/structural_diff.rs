mod common;

use common::{diff, kinds, pair};
use graft_lib::ast_diff::model::RowKind;
use graft_lib::ast_diff::render::render;
use graft_lib::ast_diff::{analyze, ChangeKind};
use graft_lib::git::ChangeStatus;

const RUST_OLD: &str = "\
use std::fmt;

/// Reads one byte.
pub fn read(input: &[u8]) -> Option<u8> {
    input.first().copied()
}

pub fn checksum(input: &[u8]) -> u32 {
    let mut sum = 0u32;
    for byte in input {
        sum = sum.wrapping_add(u32::from(*byte));
    }
    sum
}

pub fn is_empty(input: &[u8]) -> bool {
    input.is_empty()
}
";

#[test]
fn whitespace_and_comment_edits_are_not_changes() {
    let new = RUST_OLD
        .replace("/// Reads one byte.", "/// Reads a single byte from the front.")
        .replace("    let mut sum = 0u32;", "    let mut sum   =   0u32; // running total");
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert!(analysis.changes.is_empty(), "{:?}", kinds(&analysis));
    assert!(file.blocks.is_empty());
}

#[test]
fn relocated_function_is_reported_as_moved_and_nothing_else() {
    let moved = "pub fn read(input: &[u8]) -> Option<u8> {\n    input.first().copied()\n}\n";
    let new =
        format!("{}\n/// Reads one byte.\n{moved}", RUST_OLD.replace(&format!("/// Reads one byte.\n{moved}\n"), ""));
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert_eq!(kinds(&analysis), [(ChangeKind::Moved, "read".to_string())]);
    let kinds: Vec<_> = file.blocks.iter().map(|block| block.kind).collect();
    assert_eq!(kinds, [ChangeKind::Origin, ChangeKind::Moved]);
    assert_eq!(file.blocks[0].link, Some(1));
    assert_eq!(file.blocks[1].link, Some(0));
    assert_eq!(file.blocks[1].old.as_ref().map(|at| at.start), Some(3));
    assert_eq!(file.blocks[1].new.as_ref().map(|at| at.start), Some(15));
    assert!(file.blocks[1].rows.iter().all(|row| matches!(row.kind, RowKind::Context | RowKind::Fold)));
}

#[test]
fn signature_and_body_edits_are_told_apart() {
    let new = RUST_OLD
        .replace("pub fn read(input: &[u8]) -> Option<u8>", "pub fn read(input: &[u8], at: usize) -> Option<u8>")
        .replace("sum.wrapping_add(u32::from(*byte))", "sum.rotate_left(1) ^ u32::from(*byte)")
        .replace(
            "fn is_empty(input: &[u8]) -> bool {\n    input.is_empty()",
            "fn is_blank(input: &[u8]) -> bool {\n    input.iter().all(u8::is_ascii_whitespace)",
        );
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert_eq!(
        kinds(&analysis),
        [
            (ChangeKind::Signature, "read".to_string()),
            (ChangeKind::Body, "checksum".to_string()),
            (ChangeKind::Added, "is_blank".to_string()),
            (ChangeKind::Deleted, "is_empty".to_string()),
        ]
    );
    let body = file.blocks.iter().find(|block| block.name == "checksum").expect("checksum block");
    let changed: Vec<_> = body.rows.iter().filter(|row| row.kind == RowKind::Change).collect();
    assert_eq!(changed.len(), 1);
    let flagged: String = changed[0]
        .new
        .as_ref()
        .expect("new side")
        .segments
        .iter()
        .filter(|s| s.changed)
        .map(|s| s.text.as_str())
        .collect();
    assert!(flagged.contains("rotate_left"), "flagged tokens: {flagged:?}");
    assert!(!flagged.contains("sum ="), "unchanged tokens must stay unflagged: {flagged:?}");
}

#[test]
fn function_moved_to_another_file_and_edited_keeps_its_identity() {
    let lexer_new = RUST_OLD.replace(
        "pub fn checksum(input: &[u8]) -> u32 {\n    let mut sum = 0u32;\n    for byte in input {\n        sum = sum.wrapping_add(u32::from(*byte));\n    }\n    sum\n}\n\n",
        "",
    );
    let parser_new = "pub fn checksum(input: &[u8]) -> u32 {\n    let mut sum = 1u32;\n    for byte in input {\n        sum = sum.wrapping_add(u32::from(*byte));\n    }\n    sum\n}\n";
    let mut added = pair("src/sum.rs", "", parser_new);
    added.change.status = ChangeStatus::Added;
    let analysis = analyze(vec![pair("src/io.rs", RUST_OLD, &lexer_new), added]);

    assert_eq!(kinds(&analysis), [(ChangeKind::MovedEdited, "checksum".to_string())]);
    let similarity = analysis.changes[0].similarity;
    assert!((0.9..1.0).contains(&similarity), "similarity {similarity}");

    let summary = analysis.summary();
    assert_eq!(summary.files[0].changes[0].kind, ChangeKind::Origin);
    assert_eq!(summary.files[0].changes[0].other_path.as_deref(), Some("src/sum.rs"));
    assert_eq!(summary.files[1].changes[0].other_path.as_deref(), Some("src/io.rs"));

    let destination = render(&analysis, 1);
    assert_eq!(destination.blocks[0].old.as_ref().and_then(|at| at.path.as_deref()), Some("src/io.rs"));
    let origin = render(&analysis, 0);
    assert_eq!(origin.blocks[0].kind, ChangeKind::Origin);
    assert_eq!(origin.blocks[0].new.as_ref().and_then(|at| at.path.as_deref()), Some("src/sum.rs"));
}

#[test]
fn top_level_code_outside_declarations_is_reported_as_line_hunks() {
    let new = RUST_OLD.replace("use std::fmt;", "use std::fmt;\nuse std::io;");
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert!(analysis.changes.is_empty());
    assert_eq!(file.blocks.len(), 1);
    assert_eq!(file.blocks[0].kind, ChangeKind::Lines);
    assert_eq!(file.blocks[0].new.as_ref().map(|at| (at.start, at.end)), Some((1, 3)));
}

#[test]
fn edit_beside_a_nested_match_is_still_a_body_change() {
    let old = "pub fn weight(input: &[u8]) -> usize {\n    let base = input.len();\n    match input.first() {\n        Some(_) => base,\n        None => 0,\n    }\n}\n";
    let new = old.replace("let base = input.len();", "let base = input.len() * 2;");
    let (analysis, _) = diff("src/weight.rs", old, &new);

    assert_eq!(kinds(&analysis), [(ChangeKind::Body, "weight".to_string())]);
}
