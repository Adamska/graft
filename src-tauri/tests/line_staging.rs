mod common;
#[path = "common/repo.rs"]
mod repo;

use graft_lib::ast_diff::analyze;
use graft_lib::ast_diff::model::{Row, RowKind};
use graft_lib::ast_diff::render::raw_rows;
use graft_lib::ast_diff::splice::{splice, Direction, LineSelection};
use graft_lib::changeset::DiffTarget;
use graft_lib::staging;
use repo::Scratch;

const AUTH: &str = "src/auth.rs";

fn rows(scratch: &Scratch, target: &DiffTarget, path: &str) -> Vec<Row> {
    let analysis = scratch.session.analyze(target).expect("analyze");
    analysis.file_index(path).map(|file| raw_rows(&analysis.files[file])).unwrap_or_default()
}

/// One selection per run of consecutive changed rows, which is what the view calls a hunk.
fn runs(rows: &[Row]) -> Vec<LineSelection> {
    let mut runs: Vec<LineSelection> = Vec::new();
    let mut open = false;
    for row in rows {
        if matches!(row.kind, RowKind::Context | RowKind::Fold) {
            open = false;
            continue;
        }
        if !open {
            runs.push(LineSelection::default());
            open = true;
        }
        let run = runs.last_mut().expect("run is open");
        run.old.extend(row.old.as_ref().map(|line| line.number));
        run.new.extend(row.new.as_ref().map(|line| line.number));
    }
    runs
}

/// `(added, deleted)` line counts of `path` according to git itself.
fn numstat(scratch: &Scratch, cached: bool) -> (usize, usize) {
    let args: &[&str] =
        if cached { &["diff", "--cached", "--numstat", "--", AUTH] } else { &["diff", "--numstat", "--", AUTH] };
    let stat = scratch.git(args);
    let mut fields = stat.split_whitespace().map(|field| field.parse().unwrap_or(0));
    (fields.next().unwrap_or(0), fields.next().unwrap_or(0))
}

fn size(selection: &LineSelection) -> (usize, usize) {
    (selection.new.len(), selection.old.len())
}

#[test]
fn stages_one_hunk_and_leaves_the_others_unstaged() {
    let scratch = Scratch::new("lines-hunk");
    let before = runs(&rows(&scratch, &DiffTarget::Unstaged, AUTH));
    let total = numstat(&scratch, false);
    assert!(before.len() >= 3, "the demo edits several places in {AUTH}");

    let hunk = &before[1];
    staging::stage_lines(&scratch.session, AUTH, hunk).expect("stage second hunk");
    assert_eq!(numstat(&scratch, true), size(hunk));
    assert_eq!(numstat(&scratch, false), (total.0 - hunk.new.len(), total.1 - hunk.old.len()));

    let staged = runs(&rows(&scratch, &DiffTarget::Staged, AUTH));
    staging::unstage_lines(&scratch.session, AUTH, &staged[0]).expect("unstage it again");
    assert!(rows(&scratch, &DiffTarget::Staged, AUTH).is_empty());
    assert_eq!(numstat(&scratch, false), total);
}

#[test]
fn stages_a_single_added_line_out_of_a_hunk() {
    let scratch = Scratch::new("lines-single");
    let before = runs(&rows(&scratch, &DiffTarget::Unstaged, AUTH));
    let total = numstat(&scratch, false);
    let line = before.iter().find_map(|run| run.new.first().copied()).expect("an added line");

    staging::stage_lines(&scratch.session, AUTH, &LineSelection { old: Vec::new(), new: vec![line] }).expect("stage");
    assert_eq!(numstat(&scratch, true), (1, 0));
    assert_eq!(numstat(&scratch, false), (total.0 - 1, total.1));
}

#[test]
fn selecting_every_line_stages_the_whole_file() {
    let scratch = Scratch::new("lines-all");
    let mut all = LineSelection::default();
    for run in runs(&rows(&scratch, &DiffTarget::Unstaged, AUTH)) {
        all.old.extend(run.old);
        all.new.extend(run.new);
    }
    staging::stage_lines(&scratch.session, AUTH, &all).expect("stage everything");
    assert!(rows(&scratch, &DiffTarget::Unstaged, AUTH).is_empty());
    assert_eq!(scratch.git(&["diff", "--name-only", "--", AUTH]), "");
}

#[test]
fn discards_one_hunk_from_disk_only() {
    let scratch = Scratch::new("lines-discard");
    let before = runs(&rows(&scratch, &DiffTarget::Unstaged, AUTH));
    let total = numstat(&scratch, false);
    let index_before = scratch.git(&["ls-files", "--stage", "--", AUTH]);

    staging::discard_lines(&scratch.session, AUTH, &before[0]).expect("discard first hunk");
    assert_eq!(numstat(&scratch, false), (total.0 - before[0].new.len(), total.1 - before[0].old.len()));
    assert_eq!(scratch.git(&["ls-files", "--stage", "--", AUTH]), index_before, "the index must not move");
}

#[test]
fn stages_part_of_an_untracked_file() {
    let scratch = Scratch::new("lines-untracked");
    staging::stage_lines(&scratch.session, "notes/todo.md", &LineSelection { old: Vec::new(), new: vec![1] })
        .expect("stage first line");
    let staged = scratch.git(&["show", ":notes/todo.md"]);
    assert_eq!(staged, scratch.read("notes/todo.md").lines().next().expect("first line"));
}

#[test]
fn rejects_a_selection_that_no_longer_matches_the_file() {
    let scratch = Scratch::new("lines-stale");
    let stale = LineSelection { old: Vec::new(), new: vec![1] };
    let error = staging::stage_lines(&scratch.session, AUTH, &stale).expect_err("line 1 is unchanged").to_string();
    assert!(error.contains("changed since it was shown"), "unexpected message: {error}");
}

#[test]
fn keeps_line_endings_and_a_missing_final_newline() {
    let old = "alpha\r\nbeta\r\ngamma";
    let new = "alpha\r\nBETA\r\ngamma\r\ndelta";
    let analysis = analyze(vec![common::pair("notes.txt", old, new)]);
    let pair = &analysis.files[0];

    let replace = LineSelection { old: vec![2], new: vec![2] };
    assert_eq!(splice(pair, &replace, Direction::Apply).as_deref(), Some("alpha\r\nBETA\r\ngamma"));
    assert_eq!(splice(pair, &replace, Direction::Revert).as_deref(), Some("alpha\r\nbeta\r\ngamma\r\ndelta"));

    let append = LineSelection { old: Vec::new(), new: vec![4] };
    assert_eq!(splice(pair, &append, Direction::Apply).as_deref(), Some("alpha\r\nbeta\r\ngamma\ndelta"));
    assert_eq!(splice(pair, &LineSelection::default(), Direction::Apply).as_deref(), Some(old));
    assert_eq!(splice(pair, &LineSelection::default(), Direction::Revert).as_deref(), Some(new));
}
