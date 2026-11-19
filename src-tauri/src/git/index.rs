use std::path::Path;

use gix::bstr::BStr;

use super::cli;
use super::content::{read_text, Side};
use crate::error::{Error, GitResultExt, Result};

const REGULAR_FILE: &str = "100644";

pub fn stage_paths(root: &Path, paths: &[String]) -> Result<()> {
    run_with_paths(root, &["add", "--all", "--"], paths)
}

pub fn unstage_paths(root: &Path, paths: &[String]) -> Result<()> {
    let has_head = cli::run(root, &["rev-parse", "--verify", "--quiet", "HEAD"]).is_ok();
    let command: &[&str] = if has_head { &["reset", "--quiet", "--"] } else { &["rm", "--cached", "--quiet", "--"] };
    run_with_paths(root, command, paths)
}

fn run_with_paths(root: &Path, command: &[&str], paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let args: Vec<&str> = command.iter().copied().chain(paths.iter().map(String::as_str)).collect();
    cli::run(root, &args).map(drop)
}

/// Text of `path` as currently staged; empty when the file is not in the index.
pub fn staged_text(repo: &gix::Repository, path: &str) -> Result<String> {
    let index = repo.index_or_empty().git()?;
    let side = index.entry_by_path(BStr::new(path)).map_or(Side::Absent, |entry| Side::Object(entry.id));
    text_or_binary_error(repo, side, path)
}

/// Text of `path` in the `HEAD` commit; empty when it does not exist there.
pub fn head_text(repo: &gix::Repository, path: &str) -> Result<String> {
    let Ok(commit) = repo.head_commit() else {
        return Ok(String::new());
    };
    let entry = commit.tree().git()?.lookup_entry_by_path(path).git()?;
    let side = entry.map_or(Side::Absent, |entry| Side::Object(entry.object_id()));
    text_or_binary_error(repo, side, path)
}

pub fn worktree_text(repo: &gix::Repository, path: &str) -> Result<String> {
    text_or_binary_error(repo, Side::Worktree, path)
}

fn text_or_binary_error(repo: &gix::Repository, side: Side, path: &str) -> Result<String> {
    read_text(repo, side, path)?
        .ok_or_else(|| Error::invalid(format!("{path} is binary or too large to stage partially")))
}

/// Replaces the staged content of `path` without touching the working tree.
pub fn write_staged_text(root: &Path, path: &str, text: &str) -> Result<()> {
    let listing = cli::run(root, &["ls-files", "--stage", "--", path])?;
    let mode = listing.split_whitespace().next().unwrap_or(REGULAR_FILE).to_string();
    let oid = cli::run_with_input(root, &["hash-object", "-w", "--stdin", "--path", path], text.as_bytes())?;
    cli::run(root, &["update-index", "--add", "--cacheinfo", &format!("{mode},{oid},{path}")]).map(drop)
}

/// Commits the index and returns the new commit id.
pub fn commit(root: &Path, message: &str) -> Result<String> {
    if message.trim().is_empty() {
        return Err(Error::invalid("commit message is empty"));
    }
    cli::run_with_input(root, &["commit", "--quiet", "--file", "-"], message.as_bytes())?;
    cli::run(root, &["rev-parse", "HEAD"])
}

pub fn checkout(root: &Path, revision: &str, detach: bool) -> Result<()> {
    let args: &[&str] =
        if detach { &["checkout", "--quiet", "--detach", revision] } else { &["checkout", "--quiet", revision] };
    cli::run(root, args).map(drop)
}

/// Cherry-picks `oid`; on conflict the pick is aborted so the tree is never left half-applied.
pub fn cherry_pick(root: &Path, oid: &str) -> Result<()> {
    cli::run(root, &["cherry-pick", oid]).map(drop).map_err(|error| {
        let _ = cli::run(root, &["cherry-pick", "--abort"]);
        Error::Git(format!("cherry-pick stopped and was rolled back: {error}"))
    })
}
