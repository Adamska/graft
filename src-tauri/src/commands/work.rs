use tauri::State;

use crate::ast_diff::stage::EntitySelector;
use crate::error::Result;
use crate::git::index;
use crate::staging;
use crate::state::AppState;

#[tauri::command(async)]
pub fn stage_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    let session = state.session()?;
    session.invalidate_working();
    index::stage_paths(&session.root, &paths)
}

#[tauri::command(async)]
pub fn unstage_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    let session = state.session()?;
    session.invalidate_working();
    index::unstage_paths(&session.root, &paths)
}

/// Stages a single declaration; see [`staging::stage_entity`].
#[tauri::command(async)]
pub fn stage_entity(state: State<'_, AppState>, path: String, selector: EntitySelector) -> Result<()> {
    let session = state.session()?;
    staging::stage_entity(&session, &path, &selector)
}

#[tauri::command(async)]
pub fn unstage_entity(state: State<'_, AppState>, path: String, selector: EntitySelector) -> Result<()> {
    let session = state.session()?;
    staging::unstage_entity(&session, &path, &selector)
}

/// Commits the index and returns the new commit id.
#[tauri::command(async)]
pub fn commit_staged(state: State<'_, AppState>, message: String) -> Result<String> {
    let session = state.session()?;
    session.invalidate_working();
    index::commit(&session.root, &message)
}
