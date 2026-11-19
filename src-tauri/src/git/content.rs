use gix::object::tree::diff::ChangeDetached;
use gix::ObjectId;

use super::{ChangeStatus, FileChange};
use crate::error::{GitResultExt, Result};

pub const MAX_BLOB_BYTES: usize = 1 << 20;

/// Where one side of a changed file lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Absent,
    Object(ObjectId),
    Worktree,
}

#[derive(Debug, Clone)]
pub struct ChangedBlob {
    pub change: FileChange,
    pub old: Side,
    pub new: Side,
}

impl ChangedBlob {
    pub fn old_path(&self) -> &str {
        self.change.old_path.as_deref().unwrap_or(&self.change.path)
    }
}

/// Changes a commit introduces relative to its first parent.
pub fn commit_changes(repo: &gix::Repository, commit: &gix::Commit<'_>) -> Result<Vec<ChangedBlob>> {
    let tree = commit.tree().git()?;
    let parent_tree = match commit.parent_ids().next() {
        Some(parent) => Some(parent.object().git()?.peel_to_tree().git()?),
        None => None,
    };

    let mut blobs = Vec::new();
    for change in repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None).git()? {
        let blob = match change {
            ChangeDetached::Addition { location, entry_mode, id, .. } if entry_mode.is_blob_or_symlink() => {
                changed(location.to_string(), None, ChangeStatus::Added, Side::Absent, Side::Object(id))
            }
            ChangeDetached::Deletion { location, entry_mode, id, .. } if entry_mode.is_blob_or_symlink() => {
                changed(location.to_string(), None, ChangeStatus::Deleted, Side::Object(id), Side::Absent)
            }
            ChangeDetached::Modification { location, entry_mode, previous_id, id, .. }
                if entry_mode.is_blob_or_symlink() =>
            {
                changed(location.to_string(), None, ChangeStatus::Modified, Side::Object(previous_id), Side::Object(id))
            }
            ChangeDetached::Rewrite { source_location, location, entry_mode, source_id, id, copy, .. }
                if entry_mode.is_blob_or_symlink() =>
            {
                let status = if copy { ChangeStatus::Added } else { ChangeStatus::Renamed };
                let old_path = (!copy).then(|| source_location.to_string());
                changed(location.to_string(), old_path, status, Side::Object(source_id), Side::Object(id))
            }
            _ => continue,
        };
        blobs.push(blob);
    }
    blobs.sort_by(|left, right| left.change.path.cmp(&right.change.path));
    Ok(blobs)
}

pub fn changed(path: String, old_path: Option<String>, status: ChangeStatus, old: Side, new: Side) -> ChangedBlob {
    ChangedBlob { change: FileChange { path, old_path, status }, old, new }
}

/// Reads one side of a change as text. Binary and oversized content yields `None`.
pub fn read_text(repo: &gix::Repository, side: Side, path: &str) -> Result<Option<String>> {
    let bytes = match side {
        Side::Absent => return Ok(Some(String::new())),
        Side::Object(id) => {
            let mut blob = repo.find_blob(id).git()?;
            blob.take_data()
        }
        Side::Worktree => {
            let Some(workdir) = repo.workdir() else {
                return Ok(None);
            };
            match std::fs::read(workdir.join(path)) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Some(String::new())),
                Err(error) => return Err(error.into()),
            }
        }
    };
    if bytes.len() > MAX_BLOB_BYTES || bytes.iter().take(8000).any(|byte| *byte == 0) {
        return Ok(None);
    }
    Ok(Some(String::from_utf8(bytes).unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())))
}
