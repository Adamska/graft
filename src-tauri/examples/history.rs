//! Runs the structural diff engine over recent history and reports timing percentiles,
//! plus how many changed lines a reviewer reads in a line diff versus in Graft.
//!
//! cargo run --release --example history -- /path/to/repository [commits]

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_lib::ast_diff::model::{Row, RowKind};
use graft_lib::ast_diff::render::{raw_rows, render};
use graft_lib::ast_diff::{analyze, Analysis, ChangeKind};
use graft_lib::changeset::{sources, DiffTarget};
use graft_lib::git::repo;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().map_or_else(|| PathBuf::from("."), PathBuf::from);
    let limit: usize = args.next().and_then(|count| count.parse().ok()).unwrap_or(500);

    let repository = repo::open(&path)?;
    let mut timings = Vec::new();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    let (mut files, mut structural, mut bytes) = (0usize, 0usize, 0usize);
    let mut slowest: Option<(u64, String, usize, usize)> = None;
    // Changed lines (line diff, Graft) over all commits, and over commits that moved code.
    let (mut all, mut moving, mut moving_commits) = ((0usize, 0usize), (0usize, 0usize), 0usize);
    let mut refactors: Vec<(usize, usize, String)> = Vec::new();

    let mut next = repository.head_id().ok().map(|id| id.detach());
    while let Some(id) = next.filter(|_| timings.len() < limit) {
        let commit = repository.find_commit(id)?;
        next = commit.parent_ids().next().map(|parent| parent.detach());
        if commit.parent_ids().count() > 1 {
            continue;
        }
        let analysis = analyze(sources(&repository, &DiffTarget::Commit { oid: id.to_string() })?);
        timings.push(analysis.micros);
        let parsed: usize = analysis.files.iter().map(|file| file.old.source.len() + file.new.source.len()).sum();
        if slowest.as_ref().is_none_or(|(micros, ..)| analysis.micros > *micros) {
            slowest = Some((analysis.micros, id.to_string(), analysis.files.len(), parsed));
        }
        files += analysis.files.len();
        structural += analysis.files.iter().filter(|file| file.is_structural()).count();
        bytes += parsed;
        let lines = review_lines(&analysis);
        all = (all.0 + lines.0, all.1 + lines.1);
        if analysis.changes.iter().any(|change| matches!(change.kind, ChangeKind::Moved | ChangeKind::MovedEdited)) {
            moving = (moving.0 + lines.0, moving.1 + lines.1);
            moving_commits += 1;
            let summary = commit.message()?.summary().to_string();
            refactors.push((lines.0, lines.1, format!("{} {summary}", &id.to_string()[..7])));
        }
        for change in analysis.changes.iter().filter(|change| change.kind != ChangeKind::Origin) {
            *kinds.entry(format!("{:?}", change.kind)).or_default() += 1;
        }
    }

    timings.sort_unstable();
    let percentile =
        |rank: f64| timings.get(((timings.len() as f64 - 1.0) * rank).round() as usize).copied().unwrap_or(0);
    println!("commits analyzed   {}", timings.len());
    println!("files              {files} ({structural} with a grammar)");
    println!("source parsed      {:.1} MB", bytes as f64 / 1_048_576.0);
    println!("median             {:.2} ms", percentile(0.5) as f64 / 1000.0);
    println!("p95                {:.2} ms", percentile(0.95) as f64 / 1000.0);
    println!("max                {:.2} ms", percentile(1.0) as f64 / 1000.0);
    println!("total              {:.1} ms", timings.iter().sum::<u64>() as f64 / 1000.0);
    if let Some((_, id, files, parsed)) = slowest {
        println!("slowest            {} ({files} files, {:.0} KB parsed)", &id[..10], parsed as f64 / 1024.0);
    }
    for (kind, count) in kinds {
        println!("{kind:<18} {count}");
    }
    let ratio = |(line, graft): (usize, usize)| {
        format!("{line} -> {graft} ({:.0}% fewer)", 100.0 - graft as f64 * 100.0 / line.max(1) as f64)
    };
    println!("changed lines      {}", ratio(all));
    println!("in {moving_commits:>4} moving     {}", ratio(moving));
    refactors.sort_by_key(|(line, graft, _)| std::cmp::Reverse(line.saturating_sub(*graft)));
    for (line, graft, title) in refactors.iter().take(8) {
        println!("  {line:>6} -> {graft:>5}  {title}");
    }
    Ok(())
}

/// Changed lines in the raw line diff, and in Graft's structural view where a pure move
/// is collapsed to its header and only edited tokens' lines count.
fn review_lines(analysis: &Analysis) -> (usize, usize) {
    let changed = |rows: &[Row]| -> usize {
        rows.iter()
            .map(|row| match row.kind {
                RowKind::Change => 2,
                RowKind::Add | RowKind::Del => 1,
                RowKind::Context | RowKind::Fold => 0,
            })
            .sum()
    };
    let (mut line, mut graft) = (0, 0);
    for (index, pair) in analysis.files.iter().enumerate() {
        if pair.binary {
            continue;
        }
        line += changed(&raw_rows(pair));
        graft += render(analysis, index)
            .blocks
            .iter()
            .filter(|block| block.kind != ChangeKind::Moved)
            .map(|block| changed(&block.rows))
            .sum::<usize>();
    }
    (line, graft)
}
