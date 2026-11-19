use tauri::State;

use crate::ast_diff::blocks::FileDiff;
use crate::ast_diff::{render, ChangesetSummary, Row};
use crate::changeset::DiffTarget;
use crate::error::{Error, Result};
use crate::state::AppState;

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

fn unknown_file(path: &str) -> Error {
    Error::invalid(format!("{path} is not part of this changeset"))
}
