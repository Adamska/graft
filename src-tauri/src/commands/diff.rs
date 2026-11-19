use tauri::ipc::Response;
use tauri::State;

use super::parse_oid;
use crate::ast_diff::blocks::FileDiff;
use crate::ast_diff::{render, ChangesetSummary, Row};
use crate::changeset::DiffTarget;
use crate::error::{Error, Result};
use crate::git::blame::{self, Blame};
use crate::state::AppState;

const HISTORY_LIMIT: usize = 5000;

/// Parses and matches the whole changeset, then returns its per-file overview.
#[tauri::command(async)]
pub fn analyze_changeset(state: State<'_, AppState>, target: DiffTarget) -> Result<ChangesetSummary> {
    let session = state.session()?;
    let analysis = match target {
        DiffTarget::Commit { .. } => session.analysis(&target)?,
        DiffTarget::Staged | DiffTarget::Unstaged => session.analyze(&target)?,
    };
    Ok(analysis.summary())
}

#[tauri::command(async)]
pub fn file_diff(state: State<'_, AppState>, target: DiffTarget, path: String) -> Result<FileDiff> {
    let analysis = state.session()?.analysis(&target)?;
    let file = analysis.file_index(&path).ok_or_else(|| unknown_file(&path))?;
    Ok(render::render(&analysis, file))
}

#[tauri::command(async)]
pub fn raw_diff(state: State<'_, AppState>, target: DiffTarget, path: String) -> Result<Vec<Row>> {
    let analysis = state.session()?.analysis(&target)?;
    let file = analysis.file_index(&path).ok_or_else(|| unknown_file(&path))?;
    Ok(render::raw_rows(&analysis.files[file]))
}

/// Last commit to touch each line of `path`, as of the target's revision or the working tree.
#[tauri::command(async)]
pub fn blame_file(state: State<'_, AppState>, target: DiffTarget, path: String) -> Result<Blame> {
    let revision = match &target {
        DiffTarget::Commit { oid } => Some(oid.as_str()),
        DiffTarget::Staged | DiffTarget::Unstaged => None,
    };
    blame::file(&state.session()?.root, revision, &path)
}

/// Graph rows of the commits that touched `path`, as a little-endian `u32` array.
#[tauri::command(async)]
pub fn file_history(state: State<'_, AppState>, path: String) -> Result<Response> {
    let session = state.session()?;
    let oids = blame::history(&session.root, &path, HISTORY_LIMIT)?;
    let index = session.graph.read().map_err(|_| Error::invalid("the commit index is unavailable"))?;
    let rows = oids.iter().filter_map(|oid| index.row_of(&parse_oid(oid).ok()?));
    Ok(Response::new(rows.flat_map(|row| row.to_le_bytes()).collect::<Vec<u8>>()))
}

fn unknown_file(path: &str) -> Error {
    Error::invalid(format!("{path} is not part of this changeset"))
}
