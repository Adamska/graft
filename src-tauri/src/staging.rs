//! Partial staging: moves one declaration, hunk, or set of lines between the working
//! tree, the index, and `HEAD` without touching the rest of the file.

use crate::ast_diff::splice::{splice, Direction, LineSelection};
use crate::ast_diff::stage::{self, EntitySelector};
use crate::ast_diff::{self, Analysis, SourcePair};
use crate::error::{Error, Result};
use crate::git::{index, ChangeStatus, FileChange};
use crate::state::Session;

/// Stages a single declaration by rewriting only its span in the index.
pub fn stage_entity(session: &Session, path: &str, selector: &EntitySelector) -> Result<()> {
    let repo = session.repository();
    let (staged, worktree) = (index::staged_text(&repo, path)?, index::worktree_text(&repo, path)?);
    transplant(session, path, staged, worktree, selector)
}

/// Reverts a single staged declaration to its `HEAD` version, leaving the working tree alone.
/// The selector describes the staged diff, so its sides are swapped for the reverse direction.
pub fn unstage_entity(session: &Session, path: &str, selector: &EntitySelector) -> Result<()> {
    let repo = session.repository();
    let (staged, head) = (index::staged_text(&repo, path)?, index::head_text(&repo, path)?);
    let reversed = EntitySelector {
        qualified: selector.qualified.clone(),
        old_start: selector.new_start,
        new_start: selector.old_start,
    };
    transplant(session, path, staged, head, &reversed)
}

/// Stages the selected lines of the unstaged diff. Selecting every change stages the
/// file as a whole, which also covers deletions and mode changes.
pub fn stage_lines(session: &Session, path: &str, selection: &LineSelection) -> Result<()> {
    let repo = session.repository();
    let (staged, worktree) = (index::staged_text(&repo, path)?, index::worktree_text(&repo, path)?);
    let text = spliced(path, staged, worktree.clone(), selection, Direction::Apply)?;
    session.invalidate_working();
    match text == worktree {
        true => index::stage_paths(&session.root, &[path.to_string()]),
        false => index::write_staged_text(&session.root, path, &text),
    }
}

/// Takes the selected lines of the staged diff back out of the index.
pub fn unstage_lines(session: &Session, path: &str, selection: &LineSelection) -> Result<()> {
    let repo = session.repository();
    let (head, staged) = (index::head_text(&repo, path)?, index::staged_text(&repo, path)?);
    let text = spliced(path, head.clone(), staged, selection, Direction::Revert)?;
    session.invalidate_working();
    match text == head {
        true => index::unstage_paths(&session.root, &[path.to_string()]),
        false => index::write_staged_text(&session.root, path, &text),
    }
}

/// Throws the selected unstaged lines away by rewriting the file on disk.
pub fn discard_lines(session: &Session, path: &str, selection: &LineSelection) -> Result<()> {
    let repo = session.repository();
    let (staged, worktree) = (index::staged_text(&repo, path)?, index::worktree_text(&repo, path)?);
    let text = spliced(path, staged, worktree, selection, Direction::Revert)?;
    session.invalidate_working();
    Ok(std::fs::write(session.root.join(path), text)?)
}

fn spliced(path: &str, old: String, new: String, selection: &LineSelection, direction: Direction) -> Result<String> {
    let analysis = single_file(path, old, new);
    analysis
        .files
        .first()
        .and_then(|pair| splice(pair, selection, direction))
        .ok_or_else(|| Error::invalid(format!("{path} changed since it was shown; refresh and try again")))
}

fn transplant(session: &Session, path: &str, base: String, donor: String, selector: &EntitySelector) -> Result<()> {
    let analysis = single_file(path, base, donor);
    let text = stage::transplant(&analysis, selector)
        .ok_or_else(|| Error::invalid(format!("{} changed on disk; refresh and try again", selector.qualified)))?;
    session.invalidate_working();
    index::write_staged_text(&session.root, path, &text)
}

fn single_file(path: &str, old: String, new: String) -> Analysis {
    let change = FileChange { path: path.to_string(), old_path: None, status: ChangeStatus::Modified };
    ast_diff::analyze(vec![SourcePair { change, old: Some(old), new: Some(new) }])
}
