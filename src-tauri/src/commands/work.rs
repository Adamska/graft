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

/// Switches the working tree to a local branch, or detaches at any other revision.
#[tauri::command(async)]
pub fn checkout(state: State<'_, AppState>, revision: String, detach: bool) -> Result<()> {
    let session = state.session()?;
    session.invalidate_working();
    index::checkout(&session.root, &revision, detach)
}

/// Applies the changes of one commit on top of the current branch.
#[tauri::command(async)]
pub fn cherry_pick(state: State<'_, AppState>, oid: String) -> Result<()> {
    let session = state.session()?;
    session.invalidate_working();
    index::cherry_pick(&session.root, &oid)
}
