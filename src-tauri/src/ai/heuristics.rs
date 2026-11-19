use super::summary::{changed_lines, line_text};
use super::{RiskFinding, Severity};
use crate::ast_diff::blocks::Block;
use crate::ast_diff::model::{ChangeKind, RowKind};
use crate::ast_diff::render::render;
use crate::ast_diff::Analysis;

const ERROR_PATHS: [&str; 7] = ["Err(", "throw ", "raise ", "catch", "except", "if err != nil", "reject("];
const ABORTS: [&str; 6] = ["panic!(", "todo!(", "unimplemented!(", "unreachable!(", "panic(", "process.exit("];
const UNCHECKED: [&str; 5] = [".unwrap()", ".expect(", " as any", "@ts-ignore", "# type: ignore"];
const COMPARISONS: [&str; 8] = ["<", "<=", ">", ">=", "==", "!=", "===", "!=="];
const GUARDS: [&str; 4] = ["if ", "assert", "guard ", "require("];

/// Deterministic risk audit that needs no network: every rule looks only at the lines a
/// declaration actually lost or gained, so untouched code never raises a finding.
pub fn audit(analysis: &Analysis) -> Vec<RiskFinding> {
    let mut findings = Vec::new();
    for file in 0..analysis.files.len() {
        let diff = render(analysis, file);
        for block in &diff.blocks {
            let before = findings.len();
            inspect(block, &mut |severity, title: &str, detail: String| {
                findings.push(RiskFinding {
                    path: diff.path.clone(),
                    symbol: block.qualified.clone(),
                    severity,
                    title: title.to_string(),
                    detail,
                    source: "heuristic".to_string(),
                });
            });
            findings[before..].sort_by_key(|finding| finding.severity);
        }
    }
    findings
}

fn inspect(block: &Block, report: &mut impl FnMut(Severity, &str, String)) {
    if matches!(block.kind, ChangeKind::Moved | ChangeKind::Origin | ChangeKind::Lines) || block.entity.is_none() {
        return;
    }
    let callable = block.entity.is_some_and(|kind| kind.is_callable());
    let (removed, added) = (changed_lines(block, false), changed_lines(block, true));
    let count = |lines: &[String], needles: &[&str]| lines.iter().filter(|line| contains_any(line, needles)).count();
    let quote = |lines: &[String], needles: &[&str]| {
        lines.iter().find(|line| contains_any(line, needles)).map(|line| line.trim().to_string()).unwrap_or_default()
    };

    if block.kind == ChangeKind::Deleted {
        report(
            Severity::Low,
            "Declaration removed",
            format!("Any remaining reference to `{}` will no longer resolve.", block.name),
        );
        return;
    }
    if matches!(block.kind, ChangeKind::Signature | ChangeKind::Modified) && callable {
        let public = is_public(block);
        let severity = if public { Severity::High } else { Severity::Medium };
        let detail = match &block.renamed_from {
            Some(previous) => format!("`{previous}` is now `{}`; every caller must be renamed with it.", block.name),
            None => format!("Callers of `{}` must match the new parameters or return type.", block.name),
        };
        report(severity, if public { "Breaking public signature change" } else { "Signature changed" }, detail);
    }
    if block.kind == ChangeKind::Added {
        if count(&added, &UNCHECKED) + count(&added, &ABORTS) > 0 {
            let line = quote(&added, &[&UNCHECKED[..], &ABORTS[..]].concat());
            report(
                Severity::Medium,
                "New code can abort at runtime",
                format!("`{line}` fails hard instead of returning an error."),
            );
        }
        return;
    }

    let lost_error_paths = count(&removed, &ERROR_PATHS) > count(&added, &ERROR_PATHS);
    if lost_error_paths {
        let line = quote(&removed, &ERROR_PATHS);
        report(
            Severity::High,
            "Error path removed",
            format!("`{line}` no longer runs; the failure it handled is now unchecked."),
        );
    } else if count(&removed, &GUARDS) > count(&added, &GUARDS) {
        let line = quote(&removed, &GUARDS);
        report(
            Severity::Medium,
            "Guard removed",
            format!("The check `{line}` was dropped; inputs it rejected now pass through."),
        );
    }
    if count(&added, &ABORTS) > count(&removed, &ABORTS) {
        report(
            Severity::High,
            "Abort introduced",
            format!("`{}` terminates instead of recovering.", quote(&added, &ABORTS)),
        );
    } else if count(&added, &UNCHECKED) > count(&removed, &UNCHECKED) {
        let line = quote(&added, &UNCHECKED);
        report(
            Severity::Medium,
            "Unchecked failure introduced",
            format!("`{line}` bypasses error handling that the type system asked for."),
        );
    }
    if let Some((old, new)) = comparison_flip(block) {
        report(
            Severity::Medium,
            "Comparison boundary changed",
            format!("`{old}` became `{new}`; behavior differs exactly at the limit, so check the edge case."),
        );
    }
}

fn contains_any(line: &str, needles: &[&str]) -> bool {
    let trimmed = line.trim_start();
    needles.iter().any(|needle| if needle.ends_with(' ') { trimmed.starts_with(needle) } else { line.contains(needle) })
}

/// Finds an edited line whose only flagged tokens are two different comparison operators.
fn comparison_flip(block: &Block) -> Option<(String, String)> {
    block.rows.iter().filter(|row| row.kind == RowKind::Change).find_map(|row| {
        let flagged = |line: &crate::ast_diff::model::Line| -> Vec<String> {
            let changed = line.segments.iter().filter(|segment| segment.changed);
            changed.map(|segment| segment.text.trim().to_string()).filter(|text| !text.is_empty()).collect()
        };
        let (old, new) = (flagged(row.old.as_ref()?), flagged(row.new.as_ref()?));
        match (old.as_slice(), new.as_slice()) {
            ([old], [new]) if COMPARISONS.contains(&old.as_str()) && COMPARISONS.contains(&new.as_str()) => {
                Some((old.clone(), new.clone()))
            }
            _ => None,
        }
    })
}

/// Whether the declaration line carries an export marker, or a Go-style capitalized name.
fn is_public(block: &Block) -> bool {
    let declaration = block
        .rows
        .iter()
        .filter_map(|row| row.new.as_ref().or(row.old.as_ref()))
        .map(line_text)
        .find(|line| line.contains(&block.name));
    let exported = declaration.is_some_and(|line| {
        let line = line.trim_start();
        line.starts_with("pub ") || line.starts_with("export ") || line.starts_with("public ")
    });
    let go_exported = block
        .rows
        .iter()
        .any(|row| row.new.as_ref().is_some_and(|line| line_text(line).trim_start().starts_with("func ")))
        && block.name.starts_with(|first: char| first.is_ascii_uppercase());
    exported || go_exported
}
