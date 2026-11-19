use std::path::PathBuf;

use graft_lib::ast_diff::render::render;
use graft_lib::ast_diff::stage::EntitySelector;
use graft_lib::ast_diff::ChangeKind;
use graft_lib::changeset::DiffTarget;
use graft_lib::demo::sandbox;
use graft_lib::git::index;
use graft_lib::staging;
use graft_lib::state::Session;

const AUTH: &str = "src/auth.rs";

fn session(name: &str) -> (PathBuf, Session) {
    let root = std::env::temp_dir().join(format!("graft-staging-{name}-{}", std::process::id()));
    sandbox::generate(&root).expect("generate demo repository");
    let session = Session::open(&root).expect("open demo repository");
    (root, session)
}

/// Declaration-level changes of `src/auth.rs` for `target`, with a selector for each.
fn changes(session: &Session, target: &DiffTarget) -> Vec<(ChangeKind, EntitySelector)> {
    let analysis = session.analyze(target).expect("analyze");
    let Some(file) = analysis.file_index(AUTH) else { return Vec::new() };
    render(&analysis, file)
        .blocks
        .into_iter()
        .filter(|block| block.stageable && block.kind != ChangeKind::Origin)
        .map(|block| {
            let selector = EntitySelector {
                qualified: block.qualified,
                old_start: block.old.filter(|at| at.path.is_none()).map(|at| at.start),
                new_start: block.new.filter(|at| at.path.is_none()).map(|at| at.start),
            };
            (block.kind, selector)
        })
        .collect()
}

fn names(changes: &[(ChangeKind, EntitySelector)]) -> Vec<(ChangeKind, &str)> {
    changes.iter().map(|(kind, selector)| (*kind, selector.qualified.as_str())).collect()
}

fn pick<'a>(changes: &'a [(ChangeKind, EntitySelector)], name: &str) -> &'a EntitySelector {
    &changes.iter().find(|(_, selector)| selector.qualified.ends_with(name)).expect("change present").1
}

#[test]
fn stages_one_added_method_and_leaves_the_rest_unstaged() {
    let (root, session) = session("added");
    let pending = changes(&session, &DiffTarget::Unstaged);

    staging::stage_entity(&session, AUTH, pick(&pending, "remaining_ttl")).expect("stage method");

    assert_eq!(names(&changes(&session, &DiffTarget::Staged)), [(ChangeKind::Added, "AuthToken::remaining_ttl")]);
    assert_eq!(
        names(&changes(&session, &DiffTarget::Unstaged)),
        [(ChangeKind::Body, "AuthToken::is_expired"), (ChangeKind::MovedEdited, "AuthToken::validate")]
    );
    let staged = index::staged_text(&session.repository(), AUTH).expect("staged text");
    assert!(staged.contains("now > deadline"), "the unrelated body edit must not be staged");
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn staging_every_declaration_reproduces_the_working_tree_exactly() {
    let (root, session) = session("all");
    loop {
        let pending = changes(&session, &DiffTarget::Unstaged);
        let Some((_, selector)) = pending.first() else { break };
        staging::stage_entity(&session, AUTH, selector).expect("stage declaration");
    }

    let repo = session.repository();
    let staged = index::staged_text(&repo, AUTH).expect("staged text");
    let worktree = index::worktree_text(&repo, AUTH).expect("worktree text");
    assert_eq!(staged, worktree);
    assert_eq!(
        names(&changes(&session, &DiffTarget::Staged)),
        [
            (ChangeKind::Body, "AuthToken::is_expired"),
            (ChangeKind::MovedEdited, "AuthToken::validate"),
            (ChangeKind::Added, "AuthToken::remaining_ttl"),
        ]
    );
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn unstaging_a_moved_method_restores_its_head_version_in_the_index() {
    let (root, session) = session("unstage");
    let pending = changes(&session, &DiffTarget::Unstaged);
    staging::stage_entity(&session, AUTH, pick(&pending, "validate")).expect("stage moved method");
    let staged = changes(&session, &DiffTarget::Staged);
    assert_eq!(names(&staged), [(ChangeKind::MovedEdited, "AuthToken::validate")]);

    staging::unstage_entity(&session, AUTH, pick(&staged, "validate")).expect("unstage moved method");

    let repo = session.repository();
    assert!(changes(&session, &DiffTarget::Staged).is_empty());
    assert_eq!(index::staged_text(&repo, AUTH).expect("staged"), index::head_text(&repo, AUTH).expect("head"));
    assert_eq!(names(&changes(&session, &DiffTarget::Unstaged)).len(), 3);
    std::fs::remove_dir_all(root).ok();
}

#[test]
fn commits_the_staged_declaration_only() {
    let (root, session) = session("commit");
    let pending = changes(&session, &DiffTarget::Unstaged);
    staging::stage_entity(&session, AUTH, pick(&pending, "is_expired")).expect("stage body edit");
    index::unstage_paths(&session.root, &["web/api.ts".to_string()]).expect("unstage demo file");

    let oid = index::commit(&session.root, "fix(auth): expire tokens at the deadline", false).expect("commit");

    let committed = changes(&session, &DiffTarget::Commit { oid });
    assert_eq!(names(&committed), [(ChangeKind::Body, "AuthToken::is_expired")]);
    assert_eq!(changes(&session, &DiffTarget::Unstaged).len(), 2);
    std::fs::remove_dir_all(root).ok();
}
