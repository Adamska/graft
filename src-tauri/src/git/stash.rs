use std::path::Path;

use super::cli;
use crate::error::{Error, Result};

/// Shelves the working tree and index. Untracked files are included on request.
pub fn save(root: &Path, message: &str, include_untracked: bool) -> Result<()> {
    let mut args = vec!["stash", "push", "--quiet"];
    if include_untracked {
        args.push("--include-untracked");
    }
    let message = message.trim();
    if !message.is_empty() {
        args.extend(["--message", message]);
    }
    let before = cli::run(root, &["rev-parse", "--quiet", "--verify", "refs/stash"]).unwrap_or_default();
    cli::run(root, &args)?;
    let after = cli::run(root, &["rev-parse", "--quiet", "--verify", "refs/stash"]).unwrap_or_default();
    if before == after {
        return Err(Error::invalid("there are no local changes to stash"));
    }
    Ok(())
}

/// Restores a stash onto the working tree and keeps it in the list.
pub fn apply(root: &Path, index: usize) -> Result<()> {
    cli::run(root, &["stash", "apply", "--quiet", "--index", &entry(index)])
        .or_else(|_| retry(root, "apply", index))
        .map(|_| ())
}

/// Restores a stash and removes it from the list; a conflicting stash is kept.
pub fn pop(root: &Path, index: usize) -> Result<()> {
    cli::run(root, &["stash", "pop", "--quiet", "--index", &entry(index)])
        .or_else(|_| retry(root, "pop", index))
        .map(|_| ())
}

pub fn drop(root: &Path, index: usize) -> Result<()> {
    cli::run(root, &["stash", "drop", "--quiet", &entry(index)]).map(|_| ())
}

/// `--index` also restores what was staged, but fails when the index has moved on.
/// The second attempt restores everything as unstaged changes instead.
fn retry(root: &Path, verb: &str, index: usize) -> Result<String> {
    cli::run(root, &["stash", verb, "--quiet", &entry(index)])
}

fn entry(index: usize) -> String {
    format!("stash@{{{index}}}")
}
