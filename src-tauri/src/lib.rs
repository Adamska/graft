pub mod ai;
pub mod ast_diff;
pub mod changeset;
pub mod commands;
pub mod demo;
pub mod error;
pub mod git;
pub mod recent;
pub mod staging;
pub mod state;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_repository,
            commands::recent_repositories,
            commands::launch_path,
            commands::refresh_repository,
            commands::stream_graph,
            commands::search_commits,
            commands::commit_details,
            commands::working_status,
            commands::diff::analyze_changeset,
            commands::diff::file_diff,
            commands::diff::raw_diff,
            commands::work::stage_paths,
            commands::work::unstage_paths,
            commands::work::discard_paths,
            commands::work::stage_entity,
            commands::work::unstage_entity,
            commands::work::commit_staged,
            commands::work::head_message,
            commands::branch::checkout,
            commands::branch::track_branch,
            commands::branch::create_branch,
            commands::branch::rename_branch,
            commands::branch::delete_branch,
            commands::branch::create_tag,
            commands::branch::delete_tag,
            commands::branch::stash_save,
            commands::branch::stash_apply,
            commands::branch::stash_drop,
            commands::sync::branch_tracking,
            commands::sync::fetch,
            commands::sync::pull,
            commands::sync::push,
            commands::sync::push_tag,
            commands::sync::merge,
            commands::sync::rebase,
            commands::sync::cherry_pick,
            commands::sync::revert,
            commands::sync::reset,
            commands::sync::advance_operation,
            commands::sync::resolve_conflict,
            commands::assist::generate_commit_message,
            commands::assist::review_risks,
            commands::assist::assistant_status,
            commands::assist::save_assistant_settings,
            commands::create_demo_repository,
            commands::engine_metrics,
        ])
        .run(tauri::generate_context!())
        .expect("failed to launch the Graft window");
}
