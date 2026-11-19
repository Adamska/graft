use serde::Serialize;

use super::model::{ChangeKind, EntityKind, Row};

/// One-based inclusive line range; `path` is set when the range lives in another file.
#[derive(Debug, Clone, Serialize)]
pub struct Location {
    pub path: Option<String>,
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Serialize)]
pub struct Block {
    pub id: u32,
    pub kind: ChangeKind,
    pub entity: Option<EntityKind>,
    pub name: String,
    pub qualified: String,
    pub old: Option<Location>,
    pub new: Option<Location>,
    pub similarity: u8,
    pub renamed_from: Option<String>,
    /// Connects a moved block with the placeholder left at its former position.
    pub link: Option<u32>,
    pub rows: Vec<Row>,
}

#[derive(Debug, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub old_path: Option<String>,
    pub language: Option<&'static str>,
    pub structural: bool,
    pub binary: bool,
    pub blocks: Vec<Block>,
}
