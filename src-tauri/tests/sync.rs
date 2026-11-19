#[path = "common/repo.rs"]
mod repo;

use graft_lib::git::sync::{self, PullMode, Tracking};
use graft_lib::git::{branch, cli};
use repo::{commit_file, Scratch};

fn main_tracking(scratch: &Scratch) -> Tracking {
    let all = sync::tracking(&scratch.root).expect("tracking");
    all.into_iter().find(|entry| entry.branch == "main").expect("main tracks origin")
}

#[test]
fn reports_upstream_and_ahead_behind_per_branch() {
    let scratch = Scratch::new("sync-tracking");
    let all = sync::tracking(&scratch.root).expect("tracking");
    let names: Vec<_> = all.iter().map(|entry| (entry.branch.as_str(), entry.upstream.as_str())).collect();
    assert_eq!(
        names,
        [
            ("feature/token-rotation", "origin/feature/token-rotation"),
            ("hotfix/varint-overflow", "origin/hotfix/varint-overflow"),
            ("main", "origin/main"),
        ]
    );
    let main = main_tracking(&scratch);
    assert_eq!((main.ahead, main.behind, main.gone), (1, 0, false));
}

#[test]
fn push_publishes_commits_and_new_branches() {
    let scratch = Scratch::new("sync-push");
    scratch.git(&["stash", "--include-untracked", "--quiet"]);

    assert_eq!(sync::push(&scratch.root, None, false).expect("push"), "Pushed main to origin");
    assert_eq!(main_tracking(&scratch).ahead, 0);

    let published = sync::push(&scratch.root, Some("feature/parser-split"), false).expect("publish");
    assert_eq!(published, "Published feature/parser-split to origin");
    assert_eq!(cli::config(&scratch.root, "branch.feature/parser-split.remote").as_deref(), Some("origin"));
}

#[test]
fn pull_refuses_to_guess_on_divergence_then_rebases() {
    let scratch = Scratch::new("sync-pull");
    scratch.git(&["stash", "--include-untracked", "--quiet"]);
    let teammate = scratch.teammate();
    commit_file(&teammate, "notes.md", "from a teammate\n", "docs: add notes");
    cli::run(&teammate, &["push", "--quiet", "origin", "main"]).expect("teammate pushes");

    assert_eq!(sync::fetch(&scratch.root).expect("fetch"), "Fetched origin");
    let main = main_tracking(&scratch);
    assert_eq!((main.ahead, main.behind), (1, 1));

    let refused = sync::pull(&scratch.root, PullMode::FastForward).expect_err("diverged").to_string();
    assert!(refused.contains("diverged"), "unexpected message: {refused}");

    assert_eq!(sync::pull(&scratch.root, PullMode::Rebase).expect("rebase"), "Pulled 2 commits");
    let main = main_tracking(&scratch);
    assert_eq!((main.ahead, main.behind), (1, 0));
    assert_eq!(scratch.read("notes.md"), "from a teammate\n");
}

#[test]
fn force_push_is_leased_against_unseen_remote_work() {
    let scratch = Scratch::new("sync-lease");
    scratch.git(&["stash", "--include-untracked", "--quiet"]);
    let teammate = scratch.teammate();
    commit_file(&teammate, "notes.md", "unseen\n", "docs: add notes");
    cli::run(&teammate, &["push", "--quiet", "origin", "main"]).expect("teammate pushes");

    assert!(sync::push(&scratch.root, None, false).is_err(), "a plain push must be rejected");
    assert!(sync::push(&scratch.root, None, true).is_err(), "the lease must protect commits nobody fetched");
    sync::fetch(&scratch.root).expect("fetch");
    sync::push(&scratch.root, None, true).expect("force push after fetching");
    assert_eq!(main_tracking(&scratch).behind, 0);
}

#[test]
fn tracks_and_deletes_remote_branches() {
    let scratch = Scratch::new("sync-branches");
    scratch.git(&["stash", "--include-untracked", "--quiet"]);
    scratch.git(&["branch", "--quiet", "-D", "hotfix/varint-overflow"]);

    assert_eq!(branch::track(&scratch.root, "origin/hotfix/varint-overflow").expect("track"), "hotfix/varint-overflow");
    assert_eq!(scratch.git(&["symbolic-ref", "--short", "HEAD"]), "hotfix/varint-overflow");
    assert_eq!(
        scratch.git(&["rev-parse", "--abbrev-ref", "@{upstream}"]),
        "origin/hotfix/varint-overflow",
        "the new local branch must track the remote one"
    );

    branch::delete_remote(&scratch.root, "origin/feature/token-rotation").expect("delete remote branch");
    assert!(scratch.git(&["branch", "--remotes"]).lines().all(|line| !line.contains("token-rotation")));
    let gone = sync::tracking(&scratch.root).expect("tracking");
    assert!(gone.iter().any(|entry| entry.branch == "feature/token-rotation" && entry.gone));
}
