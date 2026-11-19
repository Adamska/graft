#[path = "common/repo.rs"]
mod repo;

use graft_lib::git::blame;
use repo::Scratch;

const ZERO: &str = "0000000000000000000000000000000000000000";

#[test]
fn blames_every_line_of_a_committed_file() {
    let scratch = Scratch::new("blame-commit");
    let report = blame::file(&scratch.root, Some("HEAD"), "src/lexer.rs").expect("blame");
    let covered: u32 = report.hunks.iter().map(|hunk| hunk.count).sum();
    assert_eq!(covered as usize, report.lines.len());
    assert!(report.hunks.len() > 1, "several commits shaped the lexer");
    assert!(report.hunks.windows(2).all(|pair| pair[0].start + pair[0].count == pair[1].start));
    assert!(report.hunks.iter().all(|hunk| hunk.oid != ZERO && !hunk.author.is_empty() && hunk.time > 0));
    assert!(report.hunks.iter().any(|hunk| hunk.summary.starts_with("refactor(lexer)")));
}

#[test]
fn marks_uncommitted_lines_in_the_working_tree() {
    let scratch = Scratch::new("blame-worktree");
    let report = blame::file(&scratch.root, None, "src/auth.rs").expect("blame");
    assert!(report.hunks.iter().any(|hunk| hunk.oid == ZERO), "pending edits are not committed yet");
    assert!(report.hunks.iter().any(|hunk| hunk.oid != ZERO));
}

#[test]
fn blames_a_deleted_file_as_of_the_parent() {
    let scratch = Scratch::new("blame-deleted");
    scratch.git(&["stash", "--include-untracked", "--quiet"]);
    scratch.git(&["rm", "--quiet", "tools/report.py"]);
    scratch.git(&["commit", "--quiet", "--message", "chore: drop report"]);
    let report = blame::file(&scratch.root, Some("HEAD"), "tools/report.py").expect("blame parent");
    assert!(!report.lines.is_empty());
}

#[test]
fn lists_the_commits_that_touched_a_file() {
    let scratch = Scratch::new("blame-history");
    let history = blame::history(&scratch.root, "src/lexer.rs", 100).expect("history");
    assert!(history.len() >= 3, "got {history:?}");
    assert_eq!(history[0], scratch.git(&["rev-parse", "HEAD"]), "the newest commit renamed is_separator");
    assert_eq!(blame::history(&scratch.root, "src/lexer.rs", 2).expect("history").len(), 2);
}
