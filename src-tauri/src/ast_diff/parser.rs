use tree_sitter::{Node, Parser};

use super::language::{self, LanguageSpec};
use super::model::{Entity, EntityKind, ParsedFile, Token};
use super::shape;
use super::tokens::{self, Context};

/// Parses `source` into tokens and declarations, falling back to a word lexer when no
/// grammar covers the file.
pub fn parse(source: String, path: &str, parser: &mut Parser) -> ParsedFile {
    let spec = language::for_path(path);
    let mut file = ParsedFile::new(source, spec.map(|spec| spec.name));
    let tree = spec.and_then(|spec| {
        parser.set_language(&(spec.grammar)()).ok()?;
        parser.parse(file.source.as_bytes(), None)
    });
    match (spec, tree) {
        (Some(spec), Some(tree)) => {
            Extractor { spec, file: &mut file, frames: Vec::new() }.run(tree.root_node());
            super::seal::seal(&mut file);
        }
        _ => {
            file.language = None;
            tokens::lex_plain(&mut file);
        }
    }
    file
}

struct Frame<'tree> {
    node: Node<'tree>,
    field: Option<&'tree str>,
    /// Whether declarations found directly below this node are extracted.
    scope: bool,
    entity: Option<usize>,
    body: Option<(usize, usize)>,
}

struct Extractor<'file, 'tree> {
    spec: &'static LanguageSpec,
    file: &'file mut ParsedFile,
    frames: Vec<Frame<'tree>>,
}

impl<'tree> Extractor<'_, 'tree> {
    /// Iterative pre-order walk; recursion would overflow on long operator chains.
    fn run(mut self, root: Node<'tree>) {
        let mut cursor = root.walk();
        'walk: loop {
            if self.enter(cursor.node(), cursor.field_name()) && cursor.goto_first_child() {
                continue;
            }
            loop {
                self.leave();
                if cursor.goto_next_sibling() {
                    continue 'walk;
                }
                if !cursor.goto_parent() {
                    break 'walk;
                }
            }
        }
    }

    fn enter(&mut self, node: Node<'tree>, field: Option<&'tree str>) -> bool {
        let mut frame = Frame { node, field, scope: false, entity: None, body: None };
        if node.child_count() == 0 || tokens::is_atomic(node.kind(), node.is_named()) {
            self.push_token(node, field);
            self.frames.push(frame);
            return false;
        }

        let in_scope = self.frames.last().is_none_or(|parent| parent.scope);
        let owner = self.frames.iter().rposition(|frame| frame.entity.is_some());
        let owner_entity = owner.and_then(|at| self.frames[at].entity);
        frame.scope = in_scope && self.spec.transparent.contains(&node.kind());

        if let Some(index) = in_scope.then(|| self.open_entity(node, owner_entity)).flatten() {
            frame.entity = Some(index);
            frame.scope = self.file.entities[index].is_container;
        } else if let (Some(at), Some(entity)) = (owner, owner_entity) {
            let declaration = &self.file.entities[entity];
            let distance = self.frames.len() - at;
            if declaration.body_tokens.is_none() && shape::is_body(declaration.kind, field, distance) {
                frame.body = Some((entity, self.file.tokens.len()));
            }
        }
        self.frames.push(frame);
        true
    }

    fn leave(&mut self) {
        let Some(frame) = self.frames.pop() else { return };
        let end = self.file.tokens.len();
        if let Some(index) = frame.entity {
            self.file.entities[index].tokens.end = end;
        }
        if let Some((index, start)) = frame.body {
            self.file.entities[index].body_tokens.get_or_insert(start..end);
        }
    }

    fn push_token(&mut self, node: Node<'tree>, field: Option<&'tree str>) {
        let range = node.byte_range();
        if range.is_empty() {
            return;
        }
        let parent = self.frames.last();
        let context = Context {
            kind: node.kind(),
            named: node.is_named(),
            field,
            parent_kind: parent.map_or("", |parent| parent.node.kind()),
            parent_field: parent.and_then(|parent| parent.field),
        };
        self.file.tokens.push(Token {
            start: range.start as u32,
            end: range.end as u32,
            class: tokens::classify(&context, &self.file.source[range]),
            kind: node.kind_id(),
            depth: self.frames.len() as u16,
        });
    }

    fn open_entity(&mut self, node: Node<'tree>, owner: Option<usize>) -> Option<usize> {
        let rule = self.spec.rule(node.kind())?;
        let (name, mut kind) = shape::identify(&self.file.source, node, rule)?;
        let owner_kind = owner.map(|owner| self.file.entities[owner].kind);
        if kind == EntityKind::Function && owner_kind.is_some_and(|kind| kind != EntityKind::Module) {
            kind = EntityKind::Method;
        }

        let outer = match self.frames.last() {
            Some(parent) if self.spec.wrappers.contains(&parent.node.kind()) => parent.node,
            _ => node,
        };
        let start = shape::attached_start(outer, self.spec.attributes);
        let span = shape::widen_to_lines(&self.file.source, start..outer.end_byte());
        let first_token = self.file.tokens.partition_point(|token| (token.start as usize) < span.start);
        let qualifier = match owner {
            Some(owner) => Some(self.file.entities[owner].qualified.clone()),
            None => shape::receiver_type(node).map(|receiver| self.text(receiver).to_string()),
        };

        self.file.entities.push(Entity {
            kind,
            qualified: qualifier
                .map_or_else(|| name.clone(), |prefix| format!("{prefix}{}{name}", self.spec.separator)),
            name,
            parent: owner,
            is_container: rule.container,
            lines: self.file.line_of(span.start)..self.file.line_of(span.end.saturating_sub(1)) + 1,
            span,
            tokens: first_token..first_token,
            body_tokens: None,
            depth: self.frames.len() as u16,
            hash: 0,
            signature_hash: 0,
            body_hash: 0,
            shallow_hash: 0,
        });
        Some(self.file.entities.len() - 1)
    }

    fn text(&self, node: Node<'tree>) -> &str {
        &self.file.source[node.byte_range()]
    }
}
