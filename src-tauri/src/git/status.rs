use gix::diff::index::Change as StagedChange;
use gix::status::index_worktree::Item as WorktreeItem;
use gix::status::plumbing::index_as_worktree_with_renames::Summary;
use gix::status::{Item, UntrackedFiles};
use serde::Serialize;

use super::content::{changed, ChangedBlob, Side};
use super::{ChangeStatus, FileChange};
use crate::error::{GitResultExt, Result};

#[derive(Debug, Default)]
pub struct StatusScan {
    pub staged: Vec<ChangedBlob>,
    pub unstaged: Vec<ChangedBlob>,
}

#[derive(Debug, Default, Serialize)]
pub struct WorkingStatus {
    pub staged: Vec<FileChange>,
    pub unstaged: Vec<FileChange>,
}

impl From<&StatusScan> for WorkingStatus {
    fn from(scan: &StatusScan) -> Self {
        let changes = |blobs: &[ChangedBlob]| blobs.iter().map(|blob| blob.change.clone()).collect();
        Self { staged: changes(&scan.staged), unstaged: changes(&scan.unstaged) }
    }
}

/// Compares `HEAD` with the index and the index with the working tree in one pass.
pub fn scan(repo: &gix::Repository) -> Result<StatusScan> {
    let mut scan = StatusScan::default();
    if repo.workdir().is_none() {
        return Ok(scan);
    }

    let items = repo
        .status(gix::progress::Discard)
        .git()?
        .untracked_files(UntrackedFiles::Files)
        .index_worktree_rewrites(None)
        .into_iter(None)
        .git()?;

    for item in items {
        match item.git()? {
            Item::TreeIndex(change) => scan.staged.extend(staged_blob(change)),
            Item::IndexWorktree(item) => scan.unstaged.extend(unstaged_blob(&item)),
        }
    }
    scan.staged.sort_by(|left, right| left.change.path.cmp(&right.change.path));
    scan.unstaged.sort_by(|left, right| left.change.path.cmp(&right.change.path));
    Ok(scan)
}

fn staged_blob(change: StagedChange) -> Option<ChangedBlob> {
    Some(match change {
        StagedChange::Addition { location, id, .. } => {
            changed(location.to_string(), None, ChangeStatus::Added, Side::Absent, Side::Object(id.into_owned()))
        }
        StagedChange::Deletion { location, id, .. } => {
            changed(location.to_string(), None, ChangeStatus::Deleted, Side::Object(id.into_owned()), Side::Absent)
        }
        StagedChange::Modification { location, previous_id, id, .. } => changed(
            location.to_string(),
            None,
            ChangeStatus::Modified,
            Side::Object(previous_id.into_owned()),
            Side::Object(id.into_owned()),
        ),
        StagedChange::Rewrite { source_location, source_id, location, id, copy, .. } => changed(
            location.to_string(),
            (!copy).then(|| source_location.to_string()),
            if copy { ChangeStatus::Added } else { ChangeStatus::Renamed },
            Side::Object(source_id.into_owned()),
            Side::Object(id.into_owned()),
        ),
    })
}

fn unstaged_blob(item: &WorktreeItem) -> Option<ChangedBlob> {
    let path = item.rela_path().to_string();
    let index_side = match item {
        WorktreeItem::Modification { entry, .. } => Side::Object(entry.id),
        _ => Side::Absent,
    };
    Some(match item.summary()? {
        Summary::Added | Summary::IntentToAdd => {
            changed(path, None, ChangeStatus::Untracked, Side::Absent, Side::Worktree)
        }
        Summary::Removed => changed(path, None, ChangeStatus::Deleted, index_side, Side::Absent),
        Summary::Conflict => changed(path, None, ChangeStatus::Conflicted, index_side, Side::Worktree),
        Summary::Modified | Summary::TypeChange | Summary::Renamed | Summary::Copied => {
            changed(path, None, ChangeStatus::Modified, index_side, Side::Worktree)
        }
    })
}
