//! Measures commit graph ingestion for a repository on disk.
//!
//! cargo run --release --example ingest -- /path/to/repository

use std::path::PathBuf;
use std::sync::RwLock;
use std::time::Instant;

use graft_lib::git::graph;
use graft_lib::git::repo;
use graft_lib::git::search::GraphIndex;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).map_or_else(|| PathBuf::from("."), PathBuf::from);

    let started = Instant::now();
    let repository = repo::open(&path)?;
    let snapshot = repo::snapshot(&repository)?;
    let opened = started.elapsed();

    let sync = repository.into_sync();
    let index = RwLock::new(GraphIndex::default());
    let mut first_chunk = None;
    let mut bytes = 0usize;
    let walk_started = Instant::now();
    let stats = graph::stream(&sync, &repo::graph_tips(&snapshot), &repo::ref_marks(&snapshot), &index, |frame| {
        first_chunk.get_or_insert_with(|| walk_started.elapsed());
        bytes += frame.len();
        true
    });

    let search_started = Instant::now();
    let hits = index.read().map(|index| index.search("fix", usize::MAX).len()).unwrap_or(0);
    let searched = search_started.elapsed();

    println!("repository      {}", snapshot.path);
    println!("refs            {} ({:.1?} to open and resolve)", snapshot.refs.len(), opened);
    println!("commits         {}", stats.commits);
    println!("lanes           {}", stats.lanes);
    println!("first chunk     {:.2?}", first_chunk.unwrap_or_default());
    println!("full graph      {:.2?}", std::time::Duration::from_micros(stats.micros));
    println!("wire payload    {:.1} MB", bytes as f64 / 1_048_576.0);
    println!("search \"fix\"    {hits} hits in {searched:.2?}");
    if let Some(memory) = memory_stats::memory_stats() {
        println!("resident        {:.1} MB", memory.physical_mem as f64 / 1_048_576.0);
    }
    Ok(())
}
