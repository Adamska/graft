use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// Runs `git` in `root` and returns its trimmed standard output.
///
/// Reads go through gix in-process; mutations are delegated to the user's git so hooks,
/// signing, and index extensions behave exactly as they do on the command line.
pub fn run(root: &Path, args: &[&str]) -> Result<String> {
    run_with_input(root, args, &[])
}

pub fn run_with_input(root: &Path, args: &[&str], input: &[u8]) -> Result<String> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_OPTIONAL_LOCKS", "0")
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
    let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Err(Error::Git(if message.is_empty() { format!("git {} failed", args.join(" ")) } else { message }))
}
