//! Structural diff engine: parses both sides of every changed file with tree-sitter,
//! pairs declarations across the whole changeset, and classifies each pair as moved,
//! edited, added, or deleted. Formats without a grammar fall back to a Myers line diff.

pub mod blocks;
mod inline;
pub mod language;
pub mod lines;
mod loose;
pub mod matcher;
pub mod model;
mod moves;
pub mod parser;
pub mod render;
mod seal;
mod shape;
mod similarity;
pub mod splice;
pub mod stage;
pub mod tokens;

use std::collections::HashMap;
use std::time::Instant;

use rayon::prelude::*;
use serde::Serialize;
use tree_sitter::Parser;

use crate::git::FileChange;
pub use matcher::{Change, EntityRef};
pub use model::{ChangeKind, Entity, EntityKind, ParsedFile, Row};

/// Text of both sides of a changed file; `None` marks binary or oversized content.
pub struct SourcePair {
    pub change: FileChange,
    pub old: Option<String>,
    pub new: Option<String>,
}

pub struct FilePair {
    pub change: FileChange,
    pub old: ParsedFile,
    pub new: ParsedFile,
    pub binary: bool,
}

impl FilePair {
    pub fn is_structural(&self) -> bool {
        !self.binary && (self.old.language.is_some() || self.new.language.is_some())
    }
}

pub struct Analysis {
    pub files: Vec<FilePair>,
    pub changes: Vec<Change>,
    /// Old-side counterpart of every paired new-side declaration, changed or not.
    pub origins: HashMap<EntityRef, EntityRef>,
    pub micros: u64,
}

#[derive(Debug, Serialize)]
pub struct ChangeSummary {
    pub kind: ChangeKind,
    pub entity: EntityKind,
    pub name: String,
    pub similarity: u8,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
    /// The other file involved when a declaration crossed file boundaries.
    pub other_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileSummary {
    #[serde(flatten)]
    pub change: FileChange,
    pub language: Option<&'static str>,
    pub structural: bool,
    pub binary: bool,
    pub changes: Vec<ChangeSummary>,
}

#[derive(Debug, Serialize)]
pub struct ChangesetSummary {
    pub files: Vec<FileSummary>,
    pub micros: u64,
}

/// Parses every file, and both sides of each file, in parallel, then matches
/// declarations across the whole changeset.
pub fn analyze(sources: Vec<SourcePair>) -> Analysis {
    let started = Instant::now();
    let files: Vec<FilePair> = sources
        .into_par_iter()
        .map_init(Parser::new, |parser, source| {
            let binary = source.old.is_none() || source.new.is_none();
            let old_path = source.change.old_path.clone().unwrap_or_else(|| source.change.path.clone());
            let (old_text, new_text) = (source.old.unwrap_or_default(), source.new.unwrap_or_default());
            let (old, new) = rayon::join(
                || parser::parse(old_text, &old_path, &mut Parser::new()),
                || parser::parse(new_text, &source.change.path, parser),
            );
            FilePair { old, new, change: source.change, binary }
        })
        .collect();
    let (changes, origins) = matcher::changes(&files);
    Analysis { files, changes, origins, micros: started.elapsed().as_micros() as u64 }
}

impl Analysis {
    pub fn old_entity(&self, at: EntityRef) -> &Entity {
        &self.files[at.file].old.entities[at.index]
    }

    pub fn new_entity(&self, at: EntityRef) -> &Entity {
        &self.files[at.file].new.entities[at.index]
    }

    pub fn file_index(&self, path: &str) -> Option<usize> {
        self.files.iter().position(|pair| pair.change.path == path)
    }

    pub fn summary(&self) -> ChangesetSummary {
        let mut files: Vec<FileSummary> = self
            .files
            .iter()
            .map(|pair| FileSummary {
                change: pair.change.clone(),
                language: pair.new.language.or(pair.old.language),
                structural: pair.is_structural(),
                binary: pair.binary,
                changes: Vec::new(),
            })
            .collect();

        for change in &self.changes {
            let old = change.old.map(|at| (at, self.old_entity(at)));
            let new = change.new.map(|at| (at, self.new_entity(at)));
            let crossed = matches!((old, new), (Some((old, _)), Some((new, _))) if old.file != new.file);
            let describe = |kind, entity: &Entity, other: Option<usize>| ChangeSummary {
                kind,
                entity: entity.kind,
                name: entity.qualified.clone(),
                similarity: (change.similarity * 100.0).round() as u8,
                old_line: old.map(|(_, entity)| entity.lines.start + 1),
                new_line: new.map(|(_, entity)| entity.lines.start + 1),
                other_path: other.map(|file| self.files[file].change.path.clone()),
            };
            if let Some((at, entity)) = new {
                let source = old.filter(|_| crossed).map(|(old, _)| old.file);
                files[at.file].changes.push(describe(change.kind, entity, source));
            }
            match (old, new) {
                (Some((at, entity)), None) => files[at.file].changes.push(describe(change.kind, entity, None)),
                (Some((at, entity)), Some((target, _))) if crossed => {
                    files[at.file].changes.push(describe(ChangeKind::Origin, entity, Some(target.file)));
                }
                _ => {}
            }
        }
        ChangesetSummary { files, micros: self.micros }
    }
}
