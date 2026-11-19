use std::path::Path;
use std::process::Command;
use std::sync::RwLock;

use graft_lib::git::search::GraphIndex;
use graft_lib::git::{graph, repo, status, wire, ChangeStatus};

fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "Ada Park")
        .env("GIT_AUTHOR_EMAIL", "ada@example.dev")
        .env("GIT_COMMITTER_NAME", "Ada Park")
        .env("GIT_COMMITTER_EMAIL", "ada@example.dev")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .status()
        .expect("git must be installed to run the test suite");
    assert!(status.success(), "git {args:?} failed");
}

fn commit(root: &Path, file: &str, contents: &str, message: &str, time: u32) {
    std::fs::write(root.join(file), contents).expect("write fixture file");
    let date = format!("{time} +0000");
    git(root, &["add", "."]);
    git(root, &["-c", "commit.gpgsign=false", "commit", "-q", "-m", message, "--date", &date]);
}

fn fixture(name: &str) -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("graft-test-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("create fixture directory");
    git(&root, &["init", "-q", "-b", "main"]);
    commit(&root, "a.txt", "one\n", "feat: add first file", 1_700_000_000);
    git(&root, &["checkout", "-q", "-b", "topic"]);
    commit(&root, "b.txt", "two\n", "feat: add second file", 1_700_000_100);
    git(&root, &["checkout", "-q", "main"]);
    commit(&root, "a.txt", "one\nthree\n", "fix: extend first file", 1_700_000_200);
    git(&root, &["-c", "commit.gpgsign=false", "merge", "-q", "--no-ff", "topic", "-m", "Merge branch 'topic'"]);
    root
}

fn word(frame: &[u8], index: usize) -> u32 {
    u32::from_le_bytes(frame[index * 4..index * 4 + 4].try_into().expect("four bytes"))
}

#[test]
fn streams_a_merge_history_as_one_final_frame() {
    let root = fixture("stream");
    let repository = repo::open(&root).expect("open fixture");
    let snapshot = repo::snapshot(&repository).expect("snapshot");
    let index = RwLock::new(GraphIndex::default());
    let mut frames = Vec::new();

    let stats = graph::stream(
        &repository.into_sync(),
        &repo::graph_tips(&snapshot),
        &repo::ref_marks(&snapshot),
        &index,
        |frame| {
            frames.push(frame);
            true
        },
    );

    assert_eq!(stats.commits, 4);
    assert_eq!(frames.len(), 1);
    let frame = &frames[0];
    assert_eq!(word(frame, 0), wire::MAGIC);
    assert_eq!(word(frame, 2), 4, "row count");
    assert_eq!(word(frame, 3), 4, "three first-parent edges plus one merge edge");
    assert_eq!(word(frame, 4), 0, "no edge is left open once the root is reached");
    assert_eq!(word(frame, 5), 2, "main and topic are both decorated");
    assert_eq!(word(frame, 7) & wire::FLAG_FINAL, wire::FLAG_FINAL);

    let merge_row = word(frame, wire::HEADER_WORDS);
    assert_eq!(merge_row >> 24, 2, "the newest commit is the merge");
    assert_eq!(merge_row & 0xFFFF, 0, "HEAD owns lane zero");

    let index = index.read().expect("index lock");
    assert_eq!(index.len(), 4);
    assert_eq!(index.search("SECOND file", 10), [1]);
    assert_eq!(index.search("ada park", 10).len(), 4);
    let head = snapshot.head.oid.expect("head oid");
    assert_eq!(index.search(&head[..8], 10), [0]);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn reports_staged_unstaged_and_untracked_changes() {
    let root = fixture("status");
    std::fs::write(root.join("a.txt"), "one\nthree\nfour\n").expect("modify tracked file");
    std::fs::write(root.join("b.txt"), "two!\n").expect("modify tracked file");
    std::fs::write(root.join("c.txt"), "new\n").expect("create untracked file");
    git(&root, &["add", "b.txt"]);

    let repository = repo::open(&root).expect("open fixture");
    let scan = status::scan(&repository).expect("status");

    let staged: Vec<_> = scan.staged.iter().map(|blob| (blob.change.path.as_str(), blob.change.status)).collect();
    let unstaged: Vec<_> = scan.unstaged.iter().map(|blob| (blob.change.path.as_str(), blob.change.status)).collect();
    assert_eq!(staged, [("b.txt", ChangeStatus::Modified)]);
    assert_eq!(unstaged, [("a.txt", ChangeStatus::Modified), ("c.txt", ChangeStatus::Untracked)]);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn describes_a_commit_against_its_first_parent() {
    let root = fixture("details");
    let repository = repo::open(&root).expect("open fixture");
    let snapshot = repo::snapshot(&repository).expect("snapshot");
    let topic = snapshot.refs.iter().find(|reference| reference.name == "topic").expect("topic branch");

    let details = repo::commit_details(&repository, topic.target).expect("commit details");

    assert_eq!(details.summary, "feat: add second file");
    assert_eq!(details.author, "Ada Park");
    assert_eq!(details.parents.len(), 1);
    assert_eq!(details.files.len(), 1);
    assert_eq!(details.files[0].path, "b.txt");
    assert_eq!(details.files[0].status, ChangeStatus::Added);
    std::fs::remove_dir_all(root).ok();
}
