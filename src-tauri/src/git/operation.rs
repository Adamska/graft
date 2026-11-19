use std::path::Path;

use gix::state::InProgress;
use serde::{Deserialize, Serialize};

use super::cli;
use crate::error::{Error, Result};

/// A multi-step command that stopped midway, usually on a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Merge,
    Rebase,
    CherryPick,
    Revert,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMode {
    /// Fast-forward when possible, otherwise create a merge commit.
    Auto,
    NoFastForward,
    Squash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetMode {
    Soft,
    Mixed,
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Continue,
    Skip,
    Abort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictSide {
    Ours,
    Theirs,
}

pub fn in_progress(repo: &gix::Repository) -> Option<Operation> {
    Some(match repo.state()? {
        InProgress::Merge => Operation::Merge,
        InProgress::Rebase | InProgress::RebaseInteractive | InProgress::ApplyMailboxRebase => Operation::Rebase,
        InProgress::CherryPick | InProgress::CherryPickSequence => Operation::CherryPick,
        InProgress::Revert | InProgress::RevertSequence => Operation::Revert,
        InProgress::ApplyMailbox | InProgress::Bisect => return None,
    })
}

pub fn merge(root: &Path, revision: &str, mode: MergeMode) -> Result<()> {
    let args: &[&str] = match mode {
        MergeMode::Auto => &["merge", "--quiet", "--no-edit", revision],
        MergeMode::NoFastForward => &["merge", "--quiet", "--no-edit", "--no-ff", revision],
        MergeMode::Squash => &["merge", "--quiet", "--squash", revision],
    };
    cli::run(root, args).map(drop).map_err(conflict_hint)
}

/// Replays the current branch on top of `onto`, stashing local edits around the rebase.
pub fn rebase(root: &Path, onto: &str) -> Result<()> {
    cli::run(root, &["rebase", "--quiet", "--autostash", onto]).map(drop).map_err(conflict_hint)
}

pub fn cherry_pick(root: &Path, oid: &str) -> Result<()> {
    let mut args = vec!["cherry-pick"];
    args.extend(mainline(root, oid));
    args.push(oid);
    cli::run(root, &args).map(drop).map_err(conflict_hint)
}

/// Creates a commit that undoes `oid`.
pub fn revert(root: &Path, oid: &str) -> Result<()> {
    let mut args = vec!["revert", "--no-edit"];
    args.extend(mainline(root, oid));
    args.push(oid);
    cli::run(root, &args).map(drop).map_err(conflict_hint)
}

/// Moves the current branch to `revision`. `Hard` also discards every local change.
pub fn reset(root: &Path, revision: &str, mode: ResetMode) -> Result<()> {
    let flag = match mode {
        ResetMode::Soft => "--soft",
        ResetMode::Mixed => "--mixed",
        ResetMode::Hard => "--hard",
    };
    cli::run(root, &["reset", "--quiet", flag, revision]).map(drop)
}

/// Resumes, skips the current step of, or abandons the operation in progress.
pub fn advance(root: &Path, operation: Operation, step: Step) -> Result<()> {
    let command = match operation {
        Operation::Merge => "merge",
        Operation::Rebase => "rebase",
        Operation::CherryPick => "cherry-pick",
        Operation::Revert => "revert",
    };
    let flag = match (operation, step) {
        (Operation::Merge, Step::Skip) => return Err(Error::invalid("a merge has no step to skip")),
        (_, Step::Continue) => "--continue",
        (_, Step::Skip) => "--skip",
        (_, Step::Abort) => "--abort",
    };
    cli::run(root, &[command, flag]).map(drop).map_err(|error| match step {
        Step::Continue => conflict_hint(error),
        _ => error,
    })
}

/// Settles a conflicted file by taking one side wholesale, then marks it resolved.
/// A side that deleted the file resolves to the deletion.
pub fn resolve(root: &Path, path: &str, side: ConflictSide) -> Result<()> {
    let flag = match side {
        ConflictSide::Ours => "--ours",
        ConflictSide::Theirs => "--theirs",
    };
    match cli::run(root, &["checkout", flag, "--", path]) {
        Ok(_) => cli::run(root, &["add", "--", path]).map(drop),
        Err(_) => cli::run(root, &["rm", "--quiet", "--", path]).map(drop),
    }
}

/// Merge commits need a mainline to diff against; the first parent is what the graph shows.
fn mainline(root: &Path, oid: &str) -> Option<&'static str> {
    cli::succeeds(root, &["rev-parse", "--verify", "--quiet", &format!("{oid}^2")]).then_some("--mainline=1")
}

pub(super) fn conflict_hint(error: Error) -> Error {
    let message = error.to_string();
    let conflicted = ["CONFLICT", "conflict", "unmerged", "Unmerged"].iter().any(|marker| message.contains(marker));
    match conflicted {
        true => Error::Git("stopped on conflicts; resolve the marked files, then continue or abort".to_string()),
        false => error,
    }
}
