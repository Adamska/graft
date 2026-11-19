use std::path::Path;

use gix::ObjectId;
use serde::Serialize;
use tauri::ipc::{Channel, InvokeResponseBody, Response};
use tauri::State;

use crate::ast_diff::blocks::FileDiff;
use crate::ast_diff::render;
use crate::ast_diff::{self, ChangesetSummary, Row};
use crate::changeset::{self, DiffTarget};
use crate::error::{Error, GitResultExt, Result};
use crate::git::graph::{self, GraphStats};
use crate::git::repo::{self, CommitDetails, RepoSnapshot};
use crate::git::status::{self, WorkingStatus};
use crate::state::{AppState, Session};

const DEFAULT_SEARCH_LIMIT: usize = 500;

#[derive(Debug, Serialize)]
pub struct EngineMetrics {
    pub resident_bytes: u64,
    pub threads: usize,
}

#[tauri::command(async)]
pub fn open_repository(state: State<'_, AppState>, path: String) -> Result<RepoSnapshot> {
    let session = state.activate(Session::open(Path::new(&path))?);
    Ok(session.snapshot())
}

#[tauri::command(async)]
pub fn refresh_repository(state: State<'_, AppState>) -> Result<RepoSnapshot> {
    state.session()?.refresh_snapshot()
}

/// Streams the commit graph as binary chunks; see `git::wire` for the frame layout.
#[tauri::command(async)]
pub fn stream_graph(state: State<'_, AppState>, on_chunk: Channel<InvokeResponseBody>) -> Result<GraphStats> {
    let session = state.session()?;
    let epoch = session.begin_stream();
    let snapshot = session.snapshot();

    let stats = graph::stream(
        &session.repo,
        &repo::graph_tips(&snapshot),
        &repo::ref_marks(&snapshot),
        &session.graph,
        |frame| {
            session.is_current_stream(epoch)
                && state.is_active(&session)
                && on_chunk.send(InvokeResponseBody::Raw(frame)).is_ok()
        },
    );
    Ok(stats)
}

/// Returns matching row indices as a little-endian `u32` array.
#[tauri::command(async)]
pub fn search_commits(state: State<'_, AppState>, query: String, limit: Option<usize>) -> Result<Response> {
    let session = state.session()?;
    let rows = session
        .graph
        .read()
        .map(|index| index.search(&query, limit.unwrap_or(DEFAULT_SEARCH_LIMIT)))
        .unwrap_or_default();
    let bytes: Vec<u8> = rows.iter().flat_map(|row| row.to_le_bytes()).collect();
    Ok(Response::new(bytes))
}

#[tauri::command(async)]
pub fn commit_details(state: State<'_, AppState>, oid: String) -> Result<CommitDetails> {
    let session = state.session()?;
    repo::commit_details(&session.repository(), parse_oid(&oid)?)
}

#[tauri::command(async)]
pub fn working_status(state: State<'_, AppState>) -> Result<WorkingStatus> {
    let session = state.session()?;
    let scan = status::scan(&session.repository())?;
    Ok(WorkingStatus::from(&scan))
}

/// Parses and matches the whole changeset, then returns its per-file overview.
#[tauri::command(async)]
pub fn analyze_changeset(state: State<'_, AppState>, target: DiffTarget) -> Result<ChangesetSummary> {
    let session = state.session()?;
    let sources = changeset::sources(&session.repository(), &target)?;
    let entry = session.store_analysis(target, ast_diff::analyze(sources));
    Ok(entry.1.summary())
}

#[tauri::command(async)]
pub fn file_diff(state: State<'_, AppState>, path: String) -> Result<FileDiff> {
    let entry = state.session()?.analysis()?;
    let file = entry.1.file_index(&path).ok_or_else(|| unknown_file(&path))?;
    Ok(render::render(&entry.1, file))
}

#[tauri::command(async)]
pub fn raw_diff(state: State<'_, AppState>, path: String) -> Result<Vec<Row>> {
    let entry = state.session()?.analysis()?;
    let file = entry.1.file_index(&path).ok_or_else(|| unknown_file(&path))?;
    Ok(render::raw_rows(&entry.1.files[file]))
}

#[tauri::command]
pub fn engine_metrics() -> EngineMetrics {
    EngineMetrics {
        resident_bytes: memory_stats::memory_stats().map_or(0, |stats| stats.physical_mem as u64),
        threads: rayon::current_num_threads(),
    }
}

pub fn parse_oid(hex: &str) -> Result<ObjectId> {
    ObjectId::from_hex(hex.as_bytes()).git()
}

fn unknown_file(path: &str) -> Error {
    Error::invalid(format!("{path} is not part of the analyzed changeset"))
}
