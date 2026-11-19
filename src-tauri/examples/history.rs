//! Runs the structural diff engine over recent history and reports timing percentiles.
//!
//! cargo run --release --example history -- /path/to/repository [commits]

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_lib::ast_diff::{analyze, ChangeKind};
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
    Ok(())
}
