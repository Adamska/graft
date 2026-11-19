use tauri::State;

use crate::error::Result;
use crate::git::operation::{self, ConflictSide, MergeMode, Operation, ResetMode, Step};
use crate::git::sync::{self, PullMode, Tracking};
use crate::state::AppState;

/// Upstream and ahead/behind counts of every local branch that tracks a remote one.
#[tauri::command(async)]
pub fn branch_tracking(state: State<'_, AppState>) -> Result<Vec<Tracking>> {
    sync::tracking(&state.session()?.root)
}

/// Fetches every remote and returns a one-line summary.
#[tauri::command(async)]
pub fn fetch(state: State<'_, AppState>) -> Result<String> {
    sync::fetch(&state.session()?.root)
}

#[tauri::command(async)]
pub fn pull(state: State<'_, AppState>, mode: PullMode) -> Result<String> {
    state.session()?.mutate(|root| sync::pull(root, mode))
}

/// Pushes a branch (the current one when `branch` is omitted), publishing it if needed.
#[tauri::command(async)]
pub fn push(state: State<'_, AppState>, branch: Option<String>, force: bool) -> Result<String> {
    sync::push(&state.session()?.root, branch.as_deref(), force)
}

#[tauri::command(async)]
pub fn push_tag(state: State<'_, AppState>, name: String) -> Result<String> {
    sync::push_tag(&state.session()?.root, &name)
}

/// Merges `revision` into the current branch.
#[tauri::command(async)]
pub fn merge(state: State<'_, AppState>, revision: String, mode: MergeMode) -> Result<()> {
    state.session()?.mutate(|root| operation::merge(root, &revision, mode))
}

/// Replays the current branch on top of `onto`.
#[tauri::command(async)]
pub fn rebase(state: State<'_, AppState>, onto: String) -> Result<()> {
    state.session()?.mutate(|root| operation::rebase(root, &onto))
}

/// Applies the changes of one commit on top of the current branch.
#[tauri::command(async)]
pub fn cherry_pick(state: State<'_, AppState>, oid: String) -> Result<()> {
    state.session()?.mutate(|root| operation::cherry_pick(root, &oid))
}

#[tauri::command(async)]
pub fn revert(state: State<'_, AppState>, oid: String) -> Result<()> {
    state.session()?.mutate(|root| operation::revert(root, &oid))
}

#[tauri::command(async)]
pub fn reset(state: State<'_, AppState>, revision: String, mode: ResetMode) -> Result<()> {
    state.session()?.mutate(|root| operation::reset(root, &revision, mode))
}

/// Continues, skips a step of, or aborts the merge, rebase, cherry-pick, or revert in progress.
#[tauri::command(async)]
pub fn advance_operation(state: State<'_, AppState>, operation: Operation, step: Step) -> Result<()> {
    state.session()?.mutate(|root| operation::advance(root, operation, step))
}

/// Resolves a conflicted file by taking one side in full.
#[tauri::command(async)]
pub fn resolve_conflict(state: State<'_, AppState>, path: String, side: ConflictSide) -> Result<()> {
    state.session()?.mutate(|root| operation::resolve(root, &path, side))
}
