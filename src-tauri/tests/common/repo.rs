//! Scratch repositories for the tests that drive real git commands.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use super::sandbox_repo::sandbox;
use graft_lib::git::cli;
use graft_lib::state::Session;

/// A fresh copy of the sandbox repository, removed again when dropped.
pub struct Scratch {
    pub root: PathBuf,
    pub session: Session,
}

impl Scratch {
    pub fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("graft-{name}-{}", std::process::id()));
        sandbox::generate(&root).expect("generate sandbox repository");
        let session = Session::open(&root).expect("open sandbox repository");
        Self { root: session.root.clone(), session }
    }

    pub fn git(&self, args: &[&str]) -> String {
        cli::run(&self.root, args).unwrap_or_else(|error| panic!("git {args:?}: {error}"))
    }

    pub fn write(&self, path: &str, contents: &str) {
        std::fs::write(self.root.join(path), contents).expect("write file");
    }

    pub fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.root.join(path)).expect("read file")
    }

    /// Commits everything in the working tree, leaving it clean.
    pub fn commit_all(&self, message: &str) {
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "--message", message]);
    }

    /// A second clone of the sandbox remote, standing in for a teammate.
    pub fn teammate(&self) -> PathBuf {
        let clone = self.root.with_extension("teammate");
        let origin = self.root.join(".git").join("relay-origin.git");
        std::fs::remove_dir_all(&clone).ok();
        cli::run(&self.root, &["clone", "--quiet", &origin.display().to_string(), &clone.display().to_string()])
            .expect("clone origin");
        for (key, value) in [("user.name", "Teammate"), ("user.email", "mate@relay.dev")] {
            cli::run(&clone, &["config", key, value]).expect("configure clone");
        }
        clone
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.root).ok();
        std::fs::remove_dir_all(self.root.with_extension("teammate")).ok();
    }
}

pub fn commit_file(root: &Path, path: &str, contents: &str, message: &str) {
    std::fs::write(root.join(path), contents).expect("write file");
    cli::run(root, &["add", "--", path]).expect("stage file");
    cli::run(root, &["commit", "--quiet", "--message", message]).expect("commit file");
}
