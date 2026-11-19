use tauri::State;

use crate::error::Result;
use crate::git::{branch, index, stash};
use crate::state::AppState;

/// Switches the working tree to a local branch, or detaches at any other revision.
#[tauri::command(async)]
pub fn checkout(state: State<'_, AppState>, revision: String, detach: bool) -> Result<()> {
    state.session()?.mutate(|root| index::checkout(root, &revision, detach))
}

/// Checks out the local branch that tracks a remote one, creating it when needed.
#[tauri::command(async)]
pub fn track_branch(state: State<'_, AppState>, name: String) -> Result<String> {
    state.session()?.mutate(|root| branch::track(root, &name))
}

#[tauri::command(async)]
pub fn create_branch(state: State<'_, AppState>, name: String, start: Option<String>, checkout: bool) -> Result<()> {
    state.session()?.mutate(|root| branch::create(root, &name, start.as_deref(), checkout))
}

#[tauri::command(async)]
pub fn rename_branch(state: State<'_, AppState>, from: String, to: String) -> Result<()> {
    state.session()?.mutate(|root| branch::rename(root, &from, &to))
}

/// Deletes a local branch, or a branch on its remote when `remote` is set.
#[tauri::command(async)]
pub fn delete_branch(state: State<'_, AppState>, name: String, force: bool, remote: bool) -> Result<()> {
    state.session()?.mutate(|root| match remote {
        true => branch::delete_remote(root, &name),
        false => branch::delete(root, &name, force),
    })
}

#[tauri::command(async)]
pub fn create_tag(state: State<'_, AppState>, name: String, target: String, message: Option<String>) -> Result<()> {
    state.session()?.mutate(|root| branch::create_tag(root, &name, &target, message.as_deref()))
}

#[tauri::command(async)]
pub fn delete_tag(state: State<'_, AppState>, name: String) -> Result<()> {
    state.session()?.mutate(|root| branch::delete_tag(root, &name))
}

#[tauri::command(async)]
pub fn stash_save(state: State<'_, AppState>, message: String, include_untracked: bool) -> Result<()> {
    state.session()?.mutate(|root| stash::save(root, &message, include_untracked))
}

/// Restores a stash; `pop` also removes it from the list once it applied cleanly.
#[tauri::command(async)]
pub fn stash_apply(state: State<'_, AppState>, index: usize, pop: bool) -> Result<()> {
    state.session()?.mutate(|root| match pop {
        true => stash::pop(root, index),
        false => stash::apply(root, index),
    })
}

#[tauri::command(async)]
pub fn stash_drop(state: State<'_, AppState>, index: usize) -> Result<()> {
    state.session()?.mutate(|root| stash::drop(root, index))
}
