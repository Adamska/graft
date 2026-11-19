use std::ops::Range;

use serde::{Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Class,
    Interface,
    TypeAlias,
    Module,
    Constant,
    Macro,
}

impl EntityKind {
    pub fn is_callable(self) -> bool {
        matches!(self, Self::Function | Self::Method)
    }

    /// Functions and methods are interchangeable when a declaration moves in or out of a type.
    pub fn family(self) -> Self {
        if self.is_callable() {
            Self::Function
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TokenClass {
    Plain = 0,
    Keyword = 1,
    String = 2,
    Number = 3,
    Comment = 4,
    Type = 5,
    Function = 6,
    Property = 7,
    Punctuation = 8,
    Constant = 9,
}

#[derive(Debug, Clone, Copy)]
pub struct Token {
    pub start: u32,
    pub end: u32,
    pub class: TokenClass,
    /// Grammar symbol id, part of the structural hash so `a - b` and `a + b` differ by kind too.
    pub kind: u16,
    /// Depth in the syntax tree; hashing it relative to the owning declaration captures
    /// nesting changes that leave the token text untouched, such as Python re-indentation.
    pub depth: u16,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub kind: EntityKind,
    pub name: String,
    pub qualified: String,
    pub parent: Option<usize>,
    pub is_container: bool,
    /// Byte span widened to whole lines, including attached doc comments and attributes.
    pub span: Range<usize>,
    pub lines: Range<u32>,
    pub tokens: Range<usize>,
    pub body_tokens: Option<Range<usize>>,
    pub depth: u16,
    pub hash: u64,
    pub signature_hash: u64,
    pub body_hash: u64,
    /// Hash of the declaration with nested declarations removed; equals `hash` for leaves.
    pub shallow_hash: u64,
}

#[derive(Debug, Default)]
pub struct ParsedFile {
    pub source: String,
    pub language: Option<&'static str>,
    pub line_starts: Vec<u32>,
    pub tokens: Vec<Token>,
    pub entities: Vec<Entity>,
}

impl ParsedFile {
    pub fn new(source: String, language: Option<&'static str>) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(source.bytes().enumerate().filter(|(_, byte)| *byte == b'\n').map(|(at, _)| at as u32 + 1));
        if line_starts.last().is_some_and(|start| *start as usize == source.len()) && line_starts.len() > 1 {
            line_starts.pop();
        }
        Self { source, language, line_starts, tokens: Vec::new(), entities: Vec::new() }
    }

    pub fn line_count(&self) -> u32 {
        if self.source.is_empty() {
            0
        } else {
            self.line_starts.len() as u32
        }
    }

    pub fn line_of(&self, byte: usize) -> u32 {
        self.line_starts.partition_point(|start| *start as usize <= byte).saturating_sub(1) as u32
    }

    /// Byte range of a line without its terminator.
    pub fn line_range(&self, line: u32) -> Range<usize> {
        let start = self.line_starts[line as usize] as usize;
        let end = self.line_starts.get(line as usize + 1).map_or(self.source.len(), |next| *next as usize);
        let text = &self.source.as_bytes()[start..end];
        let trimmed = text.strip_suffix(b"\n").unwrap_or(text);
        let trimmed = trimmed.strip_suffix(b"\r").unwrap_or(trimmed);
        start..start + trimmed.len()
    }

    pub fn line_text(&self, line: u32) -> &str {
        &self.source[self.line_range(line)]
    }

    pub fn token_text(&self, token: &Token) -> &str {
        &self.source[token.start as usize..token.end as usize]
    }

    pub fn children_of(&self, parent: Option<usize>) -> impl Iterator<Item = usize> + '_ {
        self.entities.iter().enumerate().filter(move |(_, entity)| entity.parent == parent).map(|(index, _)| index)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Moved,
    MovedEdited,
    Signature,
    Body,
    Modified,
    Added,
    Deleted,
    /// Line-level hunk outside any declaration, or any hunk in an unsupported format.
    Lines,
    /// Placeholder left at the position a moved declaration used to occupy.
    Origin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    Context,
    Change,
    Add,
    Del,
    Fold,
}

/// One run of text sharing a syntax class; serialized as `[text, class, changed]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub text: String,
    pub class: TokenClass,
    pub changed: bool,
}

impl Serialize for Segment {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (&self.text, self.class as u8, u8::from(self.changed)).serialize(serializer)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Line {
    pub number: u32,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub kind: RowKind,
    pub old: Option<Line>,
    pub new: Option<Line>,
    /// Number of hidden lines when `kind` is `fold`.
    pub hidden: u32,
}

impl Row {
    pub fn fold(hidden: u32) -> Self {
        Self { kind: RowKind::Fold, old: None, new: None, hidden }
    }
}
