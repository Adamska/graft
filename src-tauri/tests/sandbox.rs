use std::path::PathBuf;
use std::sync::RwLock;

use graft_lib::ast_diff::{analyze, ChangeKind};
use graft_lib::changeset::{self, DiffTarget};
use graft_lib::demo::sandbox;
use graft_lib::git::repo::RefKind;
use graft_lib::git::search::GraphIndex;
use graft_lib::git::{graph, repo, status, ChangeStatus};

fn generate(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("graft-sandbox-{name}-{}", std::process::id()));
    sandbox::generate(&root).expect("generate demo repository");
    root
}

fn changes(repository: &gix::Repository, target: &DiffTarget) -> Vec<(ChangeKind, String, Option<String>)> {
    let analysis = analyze(changeset::sources(repository, target).expect("load sources"));
    let summary = analysis.summary();
    summary
        .files
        .iter()
        .flat_map(|file| {
            file.changes.iter().map(|change| (change.kind, change.name.clone(), change.other_path.clone()))
        })
        .collect()
}

#[test]
fn generates_a_deterministic_multi_branch_history() {
    let (first, second) = (generate("a"), generate("b"));
    let repository = repo::open(&first).expect("open demo");
    let snapshot = repo::snapshot(&repository).expect("snapshot");

    let locals = snapshot.refs.iter().filter(|reference| reference.kind == RefKind::Local).count();
    let tags: Vec<_> = snapshot.refs.iter().filter(|r| r.kind == RefKind::Tag).map(|r| r.name.as_str()).collect();
    assert_eq!(locals, 5);
    assert_eq!(tags, ["v0.1.0", "v0.2.0"]);
    assert_eq!(snapshot.head.branch.as_deref(), Some("main"));

    let other = repo::snapshot(&repo::open(&second).expect("open demo")).expect("snapshot");
    assert_eq!(snapshot.head.oid, other.head.oid, "fixed timestamps must yield identical object ids");

    let index = RwLock::new(GraphIndex::default());
    let stats = graph::stream(
        &repository.into_sync(),
        &repo::graph_tips(&snapshot),
        &repo::ref_marks(&snapshot),
        &index,
        |_| true,
    );
    assert_eq!(stats.commits, 20);
    assert!(stats.lanes >= 3, "topic branches overlap in time, got {} lanes", stats.lanes);
    assert_eq!(index.read().expect("index").search("merge branch", 10).len(), 4);
    for root in [first, second] {
        std::fs::remove_dir_all(root).ok();
    }
}

#[test]
fn leaves_staged_unstaged_and_untracked_work_behind() {
    let root = generate("status");
    let repository = repo::open(&root).expect("open demo");
    let scan = status::scan(&repository).expect("status");

    let staged: Vec<_> = scan.staged.iter().map(|blob| blob.change.path.as_str()).collect();
    let unstaged: Vec<_> = scan.unstaged.iter().map(|blob| (blob.change.path.as_str(), blob.change.status)).collect();
    assert_eq!(staged, ["web/api.ts"]);
    assert_eq!(
        unstaged,
        [
            ("config/relay.yaml", ChangeStatus::Modified),
            ("notes/todo.md", ChangeStatus::Untracked),
            ("src/auth.rs", ChangeStatus::Modified),
        ]
    );

    let pending = changes(&repository, &DiffTarget::Unstaged);
    assert!(pending.contains(&(ChangeKind::Body, "AuthToken::is_expired".into(), None)), "{pending:?}");
    assert!(pending.contains(&(ChangeKind::MovedEdited, "AuthToken::validate".into(), None)), "{pending:?}");
    assert!(pending.contains(&(ChangeKind::Added, "AuthToken::remaining_ttl".into(), None)), "{pending:?}");
    assert_eq!(pending.len(), 3, "{pending:?}");

    let staged = changes(&repository, &DiffTarget::Staged);
    assert_eq!(staged, [(ChangeKind::Modified, "request".into(), None)]);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn parser_split_commit_is_read_as_moves_across_files() {
    let root = generate("split");
    let repository = repo::open(&root).expect("open demo");
    let snapshot = repo::snapshot(&repository).expect("snapshot");
    let branch = snapshot.refs.iter().find(|r| r.name == "feature/parser-split").expect("parser branch");
    let tip = repository.find_commit(branch.target).expect("tip");
    let split = tip.parent_ids().next().expect("first commit of the branch").to_string();

    let found = changes(&repository, &DiffTarget::Commit { oid: split });
    let lexer = Some("src/lexer.rs".to_string());
    for (kind, name) in [
        (ChangeKind::Moved, "Header"),
        (ChangeKind::Moved, "Frame"),
        (ChangeKind::Moved, "parse_header"),
        (ChangeKind::MovedEdited, "parse_frame"),
    ] {
        assert!(found.contains(&(kind, name.to_string(), lexer.clone())), "missing {kind:?} {name}: {found:?}");
    }
    assert_eq!(found.iter().filter(|(kind, ..)| *kind == ChangeKind::Origin).count(), 4);
    assert!(found.contains(&(ChangeKind::Added, "MAX_FRAME_BYTES".to_string(), None)), "{found:?}");
    assert_eq!(found.len(), 9, "{found:?}");
    std::fs::remove_dir_all(root).ok();
}
