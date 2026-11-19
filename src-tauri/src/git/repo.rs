use std::path::{Path, PathBuf};

use gix::hashtable::HashMap;
use gix::ObjectId;
use serde::Serialize;

use super::content;
use super::operation::{self, Operation};
use super::FileChange;
use crate::error::{GitResultExt, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefKind {
    Local,
    Remote,
    Tag,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefEntry {
    pub name: String,
    pub kind: RefKind,
    pub oid: String,
    pub is_head: bool,
    #[serde(skip)]
    pub target: ObjectId,
}

#[derive(Debug, Clone, Serialize)]
pub struct Stash {
    pub index: usize,
    pub message: String,
    pub oid: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Head {
    pub branch: Option<String>,
    pub oid: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepoSnapshot {
    pub path: String,
    pub name: String,
    pub head: Head,
    pub refs: Vec<RefEntry>,
    pub stashes: Vec<Stash>,
    /// Merge, rebase, cherry-pick, or revert that stopped and awaits the user.
    pub operation: Option<Operation>,
    #[serde(skip)]
    pub head_id: Option<ObjectId>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitDetails {
    pub oid: String,
    pub summary: String,
    pub body: String,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub parents: Vec<String>,
    pub files: Vec<FileChange>,
}

pub fn open(path: &Path) -> Result<gix::Repository> {
    gix::discover(path.canonicalize()?).git()
}

pub fn root_of(repo: &gix::Repository) -> PathBuf {
    repo.workdir().unwrap_or_else(|| repo.git_dir()).to_path_buf()
}

pub fn snapshot(repo: &gix::Repository) -> Result<RepoSnapshot> {
    let root = root_of(repo);
    let head_name = repo.head_name().git()?;
    let head_id = repo.head_id().ok().map(|id| id.detach());
    let name = root.file_name().map_or_else(|| root.display().to_string(), |name| name.to_string_lossy().into_owned());

    Ok(RepoSnapshot {
        path: root.display().to_string(),
        name,
        head: Head {
            branch: head_name.as_ref().map(|name| name.shorten().to_string()),
            oid: head_id.map(|id| id.to_string()),
        },
        refs: collect_refs(repo, head_name.as_ref())?,
        stashes: collect_stashes(repo),
        operation: operation::in_progress(repo),
        head_id,
    })
}

fn collect_refs(repo: &gix::Repository, head: Option<&gix::refs::FullName>) -> Result<Vec<RefEntry>> {
    let platform = repo.references().git()?;
    let groups = [
        (RefKind::Local, platform.local_branches().git()?),
        (RefKind::Remote, platform.remote_branches().git()?),
        (RefKind::Tag, platform.tags().git()?),
    ];

    let mut refs = Vec::new();
    for (kind, references) in groups {
        for mut reference in references.flatten() {
            let Ok(commit) = reference.peel_to_commit() else {
                continue;
            };
            let name = reference.name().shorten().to_string();
            if kind == RefKind::Remote && name.ends_with("/HEAD") {
                continue;
            }
            refs.push(RefEntry {
                name,
                kind,
                oid: commit.id.to_string(),
                is_head: head.is_some_and(|head| head.as_ref() == reference.name()),
                target: commit.id,
            });
        }
    }
    Ok(refs)
}

fn collect_stashes(repo: &gix::Repository) -> Vec<Stash> {
    let Ok(Some(reference)) = repo.try_find_reference("refs/stash") else {
        return Vec::new();
    };
    let mut log = reference.log_iter();
    let Ok(Some(lines)) = log.rev() else {
        return Vec::new();
    };
    lines
        .flatten()
        .enumerate()
        .map(|(index, line)| Stash { index, message: line.message.to_string(), oid: line.new_oid.to_string() })
        .collect()
}

/// Commits the graph walk starts from, `HEAD` first so the checked-out branch owns lane zero.
pub fn graph_tips(snapshot: &RepoSnapshot) -> Vec<ObjectId> {
    let mut tips: Vec<ObjectId> = snapshot.head_id.into_iter().collect();
    for reference in &snapshot.refs {
        if !tips.contains(&reference.target) {
            tips.push(reference.target);
        }
    }
    tips
}

pub fn ref_marks(snapshot: &RepoSnapshot) -> HashMap<ObjectId, Vec<u32>> {
    let mut marks: HashMap<ObjectId, Vec<u32>> = HashMap::default();
    for (index, reference) in snapshot.refs.iter().enumerate() {
        marks.entry(reference.target).or_default().push(index as u32);
    }
    marks
}

pub fn commit_details(repo: &gix::Repository, id: ObjectId) -> Result<CommitDetails> {
    let commit = repo.find_commit(id).git()?;
    let message = commit.message().git()?;
    let author = commit.author().git()?;

    Ok(CommitDetails {
        oid: id.to_string(),
        summary: message.summary().to_string(),
        body: message.body.map(|body| body.to_string()).unwrap_or_default().trim().to_string(),
        author: author.name.to_string(),
        email: author.email.to_string(),
        time: commit.time().git()?.seconds,
        parents: commit.parent_ids().map(|parent| parent.to_string()).collect(),
        files: content::commit_changes(repo, &commit)?.into_iter().map(|blob| blob.change).collect(),
    })
}
