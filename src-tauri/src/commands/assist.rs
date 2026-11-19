use tauri::{AppHandle, Manager, State};

use crate::ai::settings::{self, Status, StoredSettings};
use crate::ai::{reviewer, CommitDraft, RiskReport};
use crate::changeset::DiffTarget;
use crate::error::{Error, GitResultExt, Result};
use crate::state::AppState;

fn stored(app: &AppHandle) -> StoredSettings {
    app.path().app_config_dir().map(|directory| settings::load(&directory)).unwrap_or_default()
}

/// Drafts a commit message for what is staged, or for the unstaged changes when nothing is.
#[tauri::command(async)]
pub fn generate_commit_message(app: AppHandle, state: State<'_, AppState>) -> Result<CommitDraft> {
    let session = state.session()?;
    let mut analysis = session.analyze(&DiffTarget::Staged)?;
    if analysis.files.is_empty() {
        analysis = session.analyze(&DiffTarget::Unstaged)?;
    }
    if analysis.files.is_empty() {
        return Err(Error::invalid("there are no changes to describe"));
    }
    Ok(reviewer::draft_commit(&analysis, settings::resolve(&stored(&app))))
}

/// Audits a changeset for regressions; works offline and adds model findings when configured.
#[tauri::command(async)]
pub fn review_risks(app: AppHandle, state: State<'_, AppState>, target: DiffTarget) -> Result<RiskReport> {
    let analysis = state.session()?.analysis(&target)?;
    Ok(reviewer::review(&analysis, settings::resolve(&stored(&app))))
}

#[tauri::command(async)]
pub fn assistant_status(app: AppHandle) -> Status {
    settings::status(&stored(&app))
}

/// Saves the provider settings. An empty key keeps the one already stored.
#[tauri::command(async)]
pub fn save_assistant_settings(app: AppHandle, mut settings: StoredSettings) -> Result<Status> {
    let directory = app.path().app_config_dir().git()?;
    if settings.api_key.as_deref().is_none_or(|key| key.trim().is_empty()) {
        settings.api_key = settings::load(&directory).api_key.filter(|_| settings.provider.is_some());
    }
    settings::save(&directory, &settings)?;
    Ok(settings::status(&settings))
}
