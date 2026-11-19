use tauri::State;

use crate::ast_diff::splice::LineSelection;
use crate::ast_diff::stage::EntitySelector;
use crate::error::Result;
use crate::git::index;
use crate::staging;
use crate::state::AppState;

#[tauri::command(async)]
pub fn stage_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.session()?.mutate(|root| index::stage_paths(root, &paths))
}

#[tauri::command(async)]
pub fn unstage_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.session()?.mutate(|root| index::unstage_paths(root, &paths))
}

/// Restores tracked files from the index and deletes untracked ones.
#[tauri::command(async)]
pub fn discard_paths(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.session()?.mutate(|root| index::discard_paths(root, &paths))
}

/// Stages a single declaration; see [`staging::stage_entity`].
#[tauri::command(async)]
pub fn stage_entity(state: State<'_, AppState>, path: String, selector: EntitySelector) -> Result<()> {
    staging::stage_entity(&*state.session()?, &path, &selector)
}

#[tauri::command(async)]
pub fn unstage_entity(state: State<'_, AppState>, path: String, selector: EntitySelector) -> Result<()> {
    staging::unstage_entity(&*state.session()?, &path, &selector)
}

/// Stages the selected lines of the raw unstaged diff; a hunk is simply all of its lines.
#[tauri::command(async)]
pub fn stage_lines(state: State<'_, AppState>, path: String, selection: LineSelection) -> Result<()> {
    staging::stage_lines(&*state.session()?, &path, &selection)
}

#[tauri::command(async)]
pub fn unstage_lines(state: State<'_, AppState>, path: String, selection: LineSelection) -> Result<()> {
    staging::unstage_lines(&*state.session()?, &path, &selection)
}

#[tauri::command(async)]
pub fn discard_lines(state: State<'_, AppState>, path: String, selection: LineSelection) -> Result<()> {
    staging::discard_lines(&*state.session()?, &path, &selection)
}

/// Commits the index, or amends the last commit, and returns the resulting commit id.
#[tauri::command(async)]
pub fn commit_staged(state: State<'_, AppState>, message: String, amend: bool) -> Result<String> {
    state.session()?.mutate(|root| index::commit(root, &message, amend))
}

#[tauri::command(async)]
pub fn head_message(state: State<'_, AppState>) -> Result<String> {
    index::head_message(&state.session()?.root)
}
