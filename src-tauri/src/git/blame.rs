use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;
use tree_sitter::Parser;

use super::cli;
use crate::ast_diff::lines;
use crate::ast_diff::model::Line;
use crate::ast_diff::parser;
use crate::error::Result;

/// A run of consecutive lines last touched by the same commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlameHunk {
    /// All zeros for lines that are not committed yet.
    pub oid: String,
    pub author: String,
    pub time: i64,
    pub summary: String,
    /// One-based first line of the run.
    pub start: u32,
    pub count: u32,
}

#[derive(Debug, Serialize)]
pub struct Blame {
    pub hunks: Vec<BlameHunk>,
    pub lines: Vec<Line>,
}

/// Blames `path` as of `revision`, or the working tree when `None`. A file the revision
/// deleted is blamed as of its parent, which is the last place it existed.
pub fn file(root: &Path, revision: Option<&str>, path: &str) -> Result<Blame> {
    let run = |revision: Option<&str>| {
        let mut args = vec!["blame", "--porcelain"];
        args.extend(revision);
        args.extend(["--", path]);
        cli::run(root, &args)
    };
    let porcelain = match (run(revision), revision) {
        (Err(_), Some(revision)) => run(Some(&format!("{revision}^")))?,
        (result, _) => result?,
    };
    let (hunks, source) = parse(&porcelain);
    let parsed = parser::parse(source, path, &mut Parser::new());
    Ok(Blame { hunks, lines: lines::plain(&parsed) })
}

#[derive(Default, Clone)]
struct Meta {
    author: String,
    time: i64,
    summary: String,
}

/// Reads `git blame --porcelain`: each group opens with `<oid> <orig> <final> [<count>]`,
/// carries commit metadata the first time an id appears, and ends with the tab-prefixed line.
fn parse(porcelain: &str) -> (Vec<BlameHunk>, String) {
    let mut metas: HashMap<&str, Meta> = HashMap::new();
    let mut hunks: Vec<BlameHunk> = Vec::new();
    let mut source = String::new();
    let mut current: Option<(&str, u32)> = None;

    for line in porcelain.lines() {
        if let Some(text) = line.strip_prefix('\t') {
            source.push_str(text);
            source.push('\n');
            let Some((oid, number)) = current.take() else { continue };
            match hunks.last_mut() {
                Some(last) if last.oid == oid && last.start + last.count == number => last.count += 1,
                _ => hunks.push(BlameHunk {
                    oid: oid.to_string(),
                    author: String::new(),
                    time: 0,
                    summary: String::new(),
                    start: number,
                    count: 1,
                }),
            }
        } else if let Some((key, value)) = line.split_once(' ') {
            match (key, current) {
                ("author", Some((oid, _))) => metas.entry(oid).or_default().author = value.to_string(),
                ("author-time", Some((oid, _))) => metas.entry(oid).or_default().time = value.parse().unwrap_or(0),
                ("summary", Some((oid, _))) => metas.entry(oid).or_default().summary = value.to_string(),
                (oid, None) if oid.len() >= 40 && oid.bytes().all(|byte| byte.is_ascii_hexdigit()) => {
                    let number = value.split(' ').nth(1).and_then(|number| number.parse().ok()).unwrap_or(0);
                    current = Some((oid, number));
                }
                _ => {}
            }
        }
    }
    for hunk in &mut hunks {
        let meta = metas.get(hunk.oid.as_str()).cloned().unwrap_or_default();
        (hunk.author, hunk.time, hunk.summary) = (meta.author, meta.time, meta.summary);
    }
    (hunks, source)
}

/// Commits that touched `path`, newest first, following it across renames.
pub fn history(root: &Path, path: &str, limit: usize) -> Result<Vec<String>> {
    let count = format!("--max-count={limit}");
    let log = cli::run(root, &["log", "--format=%H", "--follow", &count, "--", path])?;
    Ok(log.lines().map(str::to_string).collect())
}
