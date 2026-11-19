use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// Runs `git` in `root` and returns its trimmed standard output.
///
/// Reads go through gix in-process; mutations are delegated to the user's git so hooks,
/// signing, credential helpers, and index extensions behave exactly as they do on the
/// command line. No command may open an editor or prompt on a terminal.
pub fn run(root: &Path, args: &[&str]) -> Result<String> {
    run_with_input(root, args, &[])
}

pub fn run_with_input(root: &Path, args: &[&str], input: &[u8]) -> Result<String> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_EDITOR", "true")
        .env("GIT_SEQUENCE_EDITOR", "true")
        .env("GIT_MERGE_AUTOEDIT", "no")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Error::Git(format!("could not launch git: {error}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input)?;
    }
    let output = child.wait_with_output()?;
    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout).trim_end().to_string());
    }
    // Some commands, merge among them, report conflicts on standard output.
    let message = [&output.stderr, &output.stdout]
        .into_iter()
        .map(|stream| String::from_utf8_lossy(stream).trim().to_string())
        .find(|message| !message.is_empty())
        .unwrap_or_else(|| format!("git {} failed", args.join(" ")));
    Err(Error::Git(message))
}

/// Whether `git` exits successfully; used for existence checks where failure is an answer.
pub fn succeeds(root: &Path, args: &[&str]) -> bool {
    run(root, args).is_ok()
}

/// Value of a configuration key, or `None` when it is unset.
pub fn config(root: &Path, key: &str) -> Option<String> {
    run(root, &["config", "--get", key]).ok().filter(|value| !value.is_empty())
}
