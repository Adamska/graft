//! Declaration-level staging: moves one function, method, or type between the working
//! tree, the index, and `HEAD` without touching the rest of the file.

use crate::ast_diff::stage::{self, EntitySelector};
use crate::ast_diff::{self, SourcePair};
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

fn transplant(session: &Session, path: &str, base: String, donor: String, selector: &EntitySelector) -> Result<()> {
    let change = FileChange { path: path.to_string(), old_path: None, status: ChangeStatus::Modified };
    let analysis = ast_diff::analyze(vec![SourcePair { change, old: Some(base), new: Some(donor) }]);
    let text = stage::transplant(&analysis, selector)
        .ok_or_else(|| Error::invalid(format!("{} changed on disk; refresh and try again", selector.qualified)))?;
    session.invalidate_working();
    index::write_staged_text(&session.root, path, &text)
}
