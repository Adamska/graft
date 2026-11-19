use std::path::Path;

use serde::{Deserialize, Serialize};

use super::cli;
use super::operation::conflict_hint;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PullMode {
    FastForward,
    Rebase,
    Merge,
}

/// How a local branch relates to the remote branch it tracks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Tracking {
    pub branch: String,
    pub upstream: String,
    pub ahead: u32,
    pub behind: u32,
    /// The upstream is configured but no longer exists on the remote.
    pub gone: bool,
}

pub fn remotes(root: &Path) -> Result<Vec<String>> {
    Ok(cli::run(root, &["remote"])?.lines().map(str::to_string).collect())
}

/// Upstream and ahead/behind counts of every local branch that tracks one.
pub fn tracking(root: &Path) -> Result<Vec<Tracking>> {
    let format = "--format=%(refname:short)%09%(upstream:short)%09%(upstream:track,nobracket)";
    let listing = cli::run(root, &["for-each-ref", format, "refs/heads"])?;
    Ok(listing.lines().filter_map(parse_tracking).collect())
}

fn parse_tracking(line: &str) -> Option<Tracking> {
    let mut fields = line.split('\t');
    let (branch, upstream, track) = (fields.next()?, fields.next()?, fields.next().unwrap_or_default());
    if upstream.is_empty() {
        return None;
    }
    let count = |label: &str| {
        track.split(", ").find_map(|part| part.strip_prefix(label)).and_then(|count| count.parse().ok()).unwrap_or(0)
    };
    Some(Tracking {
        branch: branch.to_string(),
        upstream: upstream.to_string(),
        ahead: count("ahead "),
        behind: count("behind "),
        gone: track == "gone",
    })
}

/// Fetches every remote, pruning branches that were deleted upstream.
pub fn fetch(root: &Path) -> Result<String> {
    let remotes = remotes(root)?;
    if remotes.is_empty() {
        return Err(Error::invalid("this repository has no remote to fetch from"));
    }
    cli::run(root, &["fetch", "--all", "--prune", "--quiet"])?;
    Ok(match remotes.as_slice() {
        [remote] => format!("Fetched {remote}"),
        remotes => format!("Fetched {} remotes", remotes.len()),
    })
}

/// Brings the current branch up to date with its upstream. Local edits are stashed
/// around a rebase or merge; a conflict leaves the operation open for the user to resolve.
pub fn pull(root: &Path, mode: PullMode) -> Result<String> {
    let args: &[&str] = match mode {
        PullMode::FastForward => &["pull", "--quiet", "--ff-only"],
        PullMode::Rebase => &["pull", "--quiet", "--rebase", "--autostash"],
        PullMode::Merge => &["pull", "--quiet", "--no-rebase", "--no-edit", "--autostash"],
    };
    let before = cli::run(root, &["rev-parse", "HEAD"]).unwrap_or_default();
    cli::run(root, args).map_err(|error| match mode {
        PullMode::FastForward if error.to_string().contains("fast-forward") => {
            Error::Git("the branch has diverged from its upstream; pull with rebase or merge instead".to_string())
        }
        _ => conflict_hint(error),
    })?;
    let after = cli::run(root, &["rev-parse", "HEAD"]).unwrap_or_default();
    if before == after {
        return Ok("Already up to date".to_string());
    }
    let count = cli::run(root, &["rev-list", "--count", &format!("{before}..{after}")]).unwrap_or_default();
    Ok(format!("Pulled {count} {}", if count == "1" { "commit" } else { "commits" }))
}

/// Pushes `branch` (the current branch when `None`) to its upstream. A branch that
/// tracks nothing is published to the default remote under the same name and starts
/// tracking it. Forcing uses a lease, so remote work nobody has fetched is never lost.
pub fn push(root: &Path, branch: Option<&str>, force: bool) -> Result<String> {
    let branch = match branch {
        Some(branch) => branch.to_string(),
        None => cli::run(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])
            .map_err(|_| Error::invalid("check out a branch before pushing; HEAD is detached"))?,
    };
    let upstream = cli::config(root, &format!("branch.{branch}.remote"))
        .zip(cli::config(root, &format!("branch.{branch}.merge")))
        .filter(|(remote, _)| remote != ".");

    let mut args = vec!["push", "--quiet"];
    if force {
        args.push("--force-with-lease");
    }
    let (remote, refspec) = match &upstream {
        Some((remote, merge)) => (remote.clone(), format!("refs/heads/{branch}:{merge}")),
        None => {
            args.push("--set-upstream");
            (default_remote(root)?, format!("refs/heads/{branch}:refs/heads/{branch}"))
        }
    };
    args.extend([remote.as_str(), refspec.as_str()]);
    cli::run(root, &args)?;
    Ok(match upstream {
        Some(_) => format!("Pushed {branch} to {remote}"),
        None => format!("Published {branch} to {remote}"),
    })
}

pub fn push_tag(root: &Path, tag: &str) -> Result<String> {
    let remote = default_remote(root)?;
    cli::run(root, &["push", "--quiet", &remote, &format!("refs/tags/{tag}")])?;
    Ok(format!("Pushed tag {tag} to {remote}"))
}

/// `origin` when it exists, otherwise the first configured remote.
pub fn default_remote(root: &Path) -> Result<String> {
    let remotes = remotes(root)?;
    remotes
        .iter()
        .find(|remote| *remote == "origin")
        .or(remotes.first())
        .cloned()
        .ok_or_else(|| Error::invalid("this repository has no remote to push to"))
}

/// Splits `origin/feature/x` into its remote and branch, honoring remotes that contain slashes.
pub fn split_remote_branch(root: &Path, name: &str) -> Result<(String, String)> {
    remotes(root)?
        .into_iter()
        .filter_map(|remote| {
            let branch = name.strip_prefix(&remote)?.strip_prefix('/')?.to_string();
            Some((remote, branch))
        })
        .max_by_key(|(remote, _)| remote.len())
        .ok_or_else(|| Error::invalid(format!("{name} does not belong to a configured remote")))
}
