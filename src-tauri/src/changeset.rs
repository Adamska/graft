use serde::Deserialize;

use crate::ast_diff::SourcePair;
use crate::commands::parse_oid;
use crate::error::{GitResultExt, Result};
use crate::git::content::{self, ChangedBlob};
use crate::git::status;

/// Upper bound on files analyzed per changeset; the remainder is listed but not parsed.
pub const MAX_FILES: usize = 400;

/// The pair of trees a diff is computed between.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffTarget {
    Commit { oid: String },
    Staged,
    Unstaged,
}

pub fn blobs(repo: &gix::Repository, target: &DiffTarget) -> Result<Vec<ChangedBlob>> {
    Ok(match target {
        DiffTarget::Commit { oid } => content::commit_changes(repo, &repo.find_commit(parse_oid(oid)?).git()?)?,
        DiffTarget::Staged => status::scan(repo)?.staged,
        DiffTarget::Unstaged => status::scan(repo)?.unstaged,
    })
}

/// Loads both sides of every changed file as text, ready for analysis.
pub fn sources(repo: &gix::Repository, target: &DiffTarget) -> Result<Vec<SourcePair>> {
    blobs(repo, target)?
        .into_iter()
        .take(MAX_FILES)
        .map(|blob| {
            Ok(SourcePair {
                old: content::read_text(repo, blob.old, blob.old_path())?,
                new: content::read_text(repo, blob.new, &blob.change.path)?,
                change: blob.change,
            })
        })
        .collect()
}
