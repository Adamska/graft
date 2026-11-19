use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::path::Path;

use super::history::{AUTHORS, PENDING, PUBLISHED, SCRIPT, TAGS};
use crate::error::Result;
use crate::git::cli;

/// 2026-09-01T00:00:00Z; fixed so every generated repository has identical object ids.
const EPOCH: u64 = 1_788_220_800;
const COMMIT_SPACING_SECONDS: u64 = 19 * 3600;

/// Recreates the demo repository at `root`: a multi-branch history imported in a single
/// `git fast-import` pass, a local remote, and staged, unstaged, and untracked changes.
pub fn generate(root: &Path) -> Result<()> {
    if root.join(".git").exists() {
        std::fs::remove_dir_all(root)?;
    }
    std::fs::create_dir_all(root)?;

    cli::run(root, &["init", "--quiet"])?;
    cli::run(root, &["symbolic-ref", "HEAD", "refs/heads/main"])?;
    cli::run_with_input(root, &["fast-import", "--quiet", "--done"], import_stream()?.as_bytes())?;
    cli::run(root, &["reset", "--quiet", "--hard", "main"])?;
    for (key, value) in [("user.name", "Relay Demo"), ("user.email", "demo@relay.dev"), ("commit.gpgsign", "false")] {
        cli::run(root, &["config", key, value])?;
    }

    publish(root)?;

    for (path, version, staged) in PENDING {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(target, version.render()?)?;
        if staged {
            cli::run(root, &["add", path])?;
        }
    }
    Ok(())
}

/// Gives the sandbox a remote that works offline: a bare repository kept inside its own
/// `.git` directory. `main` is left one commit ahead of it so there is something to push.
fn publish(root: &Path) -> Result<()> {
    let origin = root.join(".git").join("relay-origin.git").display().to_string();
    cli::run(root, &["init", "--quiet", "--bare", "--initial-branch=main", &origin])?;
    cli::run(root, &["remote", "add", "origin", &origin])?;
    let mut push = vec!["push", "--quiet", "origin", "--tags", "main~1:refs/heads/main"];
    push.extend(PUBLISHED);
    cli::run(root, &push)?;
    for branch in std::iter::once("main").chain(PUBLISHED) {
        cli::run(root, &["branch", "--quiet", &format!("--set-upstream-to=origin/{branch}"), branch])?;
    }
    Ok(())
}

/// Serializes the scripted history as a fast-import stream. Every commit carries its
/// full tree (`deleteall` followed by each file), which makes merge results explicit.
fn import_stream() -> Result<String> {
    let mut stream = String::new();
    let mut snapshots: HashMap<&str, BTreeMap<&str, String>> = HashMap::new();
    let mut marks: HashMap<&str, usize> = HashMap::new();

    for (index, commit) in SCRIPT.iter().enumerate() {
        let mark = index + 1;
        let (name, email) = AUTHORS[commit.author];
        let when = EPOCH + index as u64 * COMMIT_SPACING_SECONDS;
        let mut tree = commit.parents.first().and_then(|parent| snapshots.get(parent)).cloned().unwrap_or_default();
        for (path, version) in commit.files {
            tree.insert(path, version.render()?);
        }

        let _ = writeln!(stream, "commit refs/heads/{}\nmark :{mark}", commit.branch);
        let _ = writeln!(stream, "author {name} <{email}> {when} +0000\ncommitter {name} <{email}> {when} +0000");
        let _ = writeln!(stream, "data {}\n{}", commit.message.len(), commit.message);
        for (position, parent) in commit.parents.iter().filter_map(|parent| marks.get(parent)).enumerate() {
            let keyword = if position == 0 { "from" } else { "merge" };
            let _ = writeln!(stream, "{keyword} :{parent}");
        }
        stream.push_str("deleteall\n");
        for (path, contents) in &tree {
            let _ = writeln!(stream, "M 100644 inline {path}\ndata {}\n{contents}", contents.len());
        }
        stream.push('\n');
        for (_, tag) in TAGS.iter().filter(|(label, _)| *label == commit.label) {
            let _ = writeln!(stream, "reset refs/tags/{tag}\nfrom :{mark}\n");
        }

        snapshots.insert(commit.label, tree);
        marks.insert(commit.label, mark);
    }
    stream.push_str("done\n");
    Ok(stream)
}
