use std::path::Path;

use super::{cli, sync};
use crate::error::{Error, Result};

/// Creates a branch at `start` (`HEAD` when `None`) and optionally switches to it.
pub fn create(root: &Path, name: &str, start: Option<&str>, checkout: bool) -> Result<()> {
    let name = valid_name(root, name)?;
    let mut args = if checkout { vec!["checkout", "--quiet", "-b", name] } else { vec!["branch", "--quiet", name] };
    args.extend(start);
    cli::run(root, &args).map(drop)
}

pub fn rename(root: &Path, from: &str, to: &str) -> Result<()> {
    cli::run(root, &["branch", "--move", from, valid_name(root, to)?]).map(drop)
}

/// Deletes a local branch. Without `force`, git refuses when the branch is not merged.
pub fn delete(root: &Path, name: &str, force: bool) -> Result<()> {
    cli::run(root, &["branch", "--quiet", if force { "-D" } else { "-d" }, name]).map(drop).map_err(|error| match error
        .to_string()
        .contains("not fully merged")
    {
        true => Error::Git(format!("{name} has commits that are not merged anywhere; force delete to drop them")),
        false => error,
    })
}

/// Deletes `origin/name` on the remote itself.
pub fn delete_remote(root: &Path, name: &str) -> Result<()> {
    let (remote, branch) = sync::split_remote_branch(root, name)?;
    cli::run(root, &["push", "--quiet", &remote, "--delete", &branch]).map(drop)
}

/// Checks out the local counterpart of a remote branch, creating it to track the remote
/// when it does not exist yet. Returns the local branch name.
pub fn track(root: &Path, name: &str) -> Result<String> {
    let (_, branch) = sync::split_remote_branch(root, name)?;
    if cli::succeeds(root, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")]) {
        cli::run(root, &["checkout", "--quiet", &branch])?;
    } else {
        cli::run(root, &["checkout", "--quiet", "--track", "-b", &branch, name])?;
    }
    Ok(branch)
}

/// Creates a lightweight tag, or an annotated one when a message is given.
pub fn create_tag(root: &Path, name: &str, target: &str, message: Option<&str>) -> Result<()> {
    let name = valid_name(root, name)?;
    match message.map(str::trim).filter(|message| !message.is_empty()) {
        Some(message) => cli::run(root, &["tag", "--annotate", "--message", message, name, target]),
        None => cli::run(root, &["tag", name, target]),
    }
    .map(drop)
}

pub fn delete_tag(root: &Path, name: &str) -> Result<()> {
    cli::run(root, &["tag", "--delete", name]).map(drop)
}

fn valid_name<'a>(root: &Path, name: &'a str) -> Result<&'a str> {
    let name = name.trim();
    if name.is_empty() || name.starts_with('-') || !cli::succeeds(root, &["check-ref-format", "--branch", name]) {
        return Err(Error::invalid(format!("\"{name}\" is not a valid name")));
    }
    Ok(name)
}
