#[path = "common/repo.rs"]
mod repo;
mod sandbox_repo;

use graft_lib::git::operation::{self, ConflictSide, MergeMode, Operation, ResetMode, Step};
use graft_lib::git::{branch, stash, status, ChangeStatus};
use repo::Scratch;

const README: &str = "README.md";

/// A clean sandbox where `main` and `topic` both rewrote the first line of the readme.
fn diverged(name: &str) -> Scratch {
    let scratch = Scratch::new(name);
    scratch.git(&["stash", "--include-untracked", "--quiet"]);
    scratch.git(&["stash", "drop", "--quiet"]);
    let body = scratch.read(README);
    let rest = body.split_once('\n').expect("readme has several lines").1;

    branch::create(&scratch.root, "topic", None, true).expect("create topic");
    scratch.write(README, &format!("# Relay, from topic\n{rest}"));
    scratch.commit_all("docs: retitle on topic");
    scratch.git(&["checkout", "--quiet", "main"]);
    scratch.write(README, &format!("# Relay, from main\n{rest}"));
    scratch.commit_all("docs: retitle on main");
    scratch
}

fn operation(scratch: &Scratch) -> Option<Operation> {
    scratch.session.refresh_snapshot().expect("snapshot").operation
}

fn conflicts(scratch: &Scratch) -> Vec<String> {
    let scan = status::scan(&scratch.session.repository()).expect("status");
    let conflicted = scan.unstaged.into_iter().filter(|blob| blob.change.status == ChangeStatus::Conflicted);
    conflicted.map(|blob| blob.change.path).collect()
}

#[test]
fn merge_stops_on_a_conflict_and_finishes_after_resolving() {
    let scratch = diverged("op-merge");
    let stopped = operation::merge(&scratch.root, "topic", MergeMode::Auto).expect_err("conflict").to_string();
    assert!(stopped.contains("resolve the marked files"), "unexpected message: {stopped}");
    assert_eq!(operation(&scratch), Some(Operation::Merge));
    assert_eq!(conflicts(&scratch), [README]);

    operation::resolve(&scratch.root, README, ConflictSide::Theirs).expect("take theirs");
    assert!(conflicts(&scratch).is_empty());
    operation::advance(&scratch.root, Operation::Merge, Step::Continue).expect("conclude merge");

    assert_eq!(operation(&scratch), None);
    assert!(scratch.read(README).starts_with("# Relay, from topic\n"));
    assert_eq!(scratch.git(&["rev-list", "--count", "--merges", "-1", "HEAD"]), "1");
}

#[test]
fn aborting_a_merge_restores_the_branch() {
    let scratch = diverged("op-abort");
    let before = scratch.git(&["rev-parse", "HEAD"]);
    operation::merge(&scratch.root, "topic", MergeMode::NoFastForward).expect_err("conflict");
    operation::advance(&scratch.root, Operation::Merge, Step::Abort).expect("abort");
    assert_eq!(operation(&scratch), None);
    assert_eq!(scratch.git(&["rev-parse", "HEAD"]), before);
    assert_eq!(scratch.git(&["status", "--porcelain"]), "");
}

#[test]
fn rebase_stops_then_continues_with_our_side() {
    let scratch = diverged("op-rebase");
    scratch.git(&["checkout", "--quiet", "topic"]);
    operation::rebase(&scratch.root, "main").expect_err("conflict");
    assert_eq!(operation(&scratch), Some(Operation::Rebase));

    // During a rebase "ours" is the branch being rebased onto.
    operation::resolve(&scratch.root, README, ConflictSide::Theirs).expect("keep the topic line");
    operation::advance(&scratch.root, Operation::Rebase, Step::Continue).expect("continue");
    assert_eq!(operation(&scratch), None);
    assert_eq!(scratch.git(&["merge-base", "HEAD", "main"]), scratch.git(&["rev-parse", "main"]));
    assert!(scratch.read(README).starts_with("# Relay, from topic\n"));
}

#[test]
fn cherry_pick_and_revert_round_trip() {
    let scratch = diverged("op-pick");
    let base = scratch.git(&["rev-parse", "HEAD"]);
    let rotation = scratch.git(&["rev-parse", "feature/token-rotation~1"]);

    operation::cherry_pick(&scratch.root, &rotation).expect("cherry-pick");
    assert!(scratch.root.join("tests/auth_rotation.rs").exists());
    operation::revert(&scratch.root, "HEAD").expect("revert");
    assert!(!scratch.root.join("tests/auth_rotation.rs").exists());

    operation::reset(&scratch.root, &base, ResetMode::Hard).expect("reset");
    assert_eq!(scratch.git(&["rev-parse", "HEAD"]), base);
}

#[test]
fn reverting_a_merge_commit_uses_its_first_parent() {
    let scratch = diverged("op-revert-merge");
    let merge = scratch.git(&["rev-list", "--merges", "-1", "HEAD"]);
    operation::revert(&scratch.root, &merge).expect("revert merge");
    assert!(scratch.git(&["log", "-1", "--format=%s"]).starts_with("Revert \"Merge branch"));
}

#[test]
fn stashes_apply_pop_and_drop() {
    let scratch = Scratch::new("op-stash");
    stash::save(&scratch.root, "wip: parked", true).expect("stash");
    assert_eq!(scratch.git(&["status", "--porcelain"]), "");
    assert!(stash::save(&scratch.root, "", false).is_err(), "nothing left to stash");

    let stashes = scratch.session.refresh_snapshot().expect("snapshot").stashes;
    assert_eq!(stashes.len(), 1);
    assert!(stashes[0].message.ends_with("wip: parked"));

    stash::apply(&scratch.root, 0).expect("apply");
    let restored = scratch.git(&["status", "--porcelain"]);
    assert!(restored.contains("M  web/api.ts"), "the staged file must be staged again:\n{restored}");
    assert!(restored.contains("?? notes/"), "untracked files must come back:\n{restored}");

    scratch.git(&["reset", "--quiet", "--hard"]);
    scratch.git(&["clean", "--quiet", "--force", "-d"]);
    stash::pop(&scratch.root, 0).expect("pop");
    assert!(scratch.session.refresh_snapshot().expect("snapshot").stashes.is_empty());

    stash::save(&scratch.root, "", true).expect("stash again");
    stash::drop(&scratch.root, 0).expect("drop");
    assert_eq!(scratch.git(&["stash", "list"]), "");
}

#[test]
fn manages_branches_and_tags() {
    let scratch = Scratch::new("op-refs");
    branch::create(&scratch.root, "spike", Some("v0.1.0"), false).expect("create");
    assert_eq!(scratch.git(&["rev-parse", "spike"]), scratch.git(&["rev-parse", "v0.1.0^{commit}"]));
    branch::rename(&scratch.root, "spike", "spike/renamed").expect("rename");
    assert!(branch::create(&scratch.root, "bad name", None, false).is_err());
    assert!(branch::create(&scratch.root, "--force", None, false).is_err());

    let unmerged = branch::delete(&scratch.root, "feature/parser-split", false);
    assert!(unmerged.is_ok(), "the parser branch is merged into main: {unmerged:?}");
    branch::delete(&scratch.root, "spike/renamed", false).expect("delete a merged branch");

    branch::create_tag(&scratch.root, "v0.3.0", "HEAD", Some("Third release")).expect("annotated tag");
    assert_eq!(scratch.git(&["cat-file", "-t", "v0.3.0"]), "tag");
    branch::create_tag(&scratch.root, "nightly", "HEAD~1", None).expect("lightweight tag");
    assert_eq!(scratch.git(&["cat-file", "-t", "nightly"]), "commit");
    branch::delete_tag(&scratch.root, "nightly").expect("delete tag");
    assert!(scratch.git(&["tag", "--list"]).lines().all(|tag| tag != "nightly"));
}
