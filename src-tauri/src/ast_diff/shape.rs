//! Grammar-specific knowledge about how declarations are named and delimited.

use std::ops::Range;

use tree_sitter::Node;

use super::language::EntityRule;
use super::model::EntityKind;

/// Resolves a declaration's name and refines its kind, or `None` when the node is not
/// a declaration after all (a `const` that does not hold a function, for example).
pub fn identify(source: &str, node: Node<'_>, rule: &EntityRule) -> Option<(String, EntityKind)> {
    let named = |node: Node<'_>| node.child_by_field_name("name").map(|name| source[name.byte_range()].to_string());
    match node.kind() {
        "impl_item" => {
            let body = node.child_by_field_name("body").map_or(node.end_byte(), |body| body.start_byte());
            let header = source[node.start_byte()..body].trim_start_matches("unsafe").trim();
            let header = header.strip_prefix("impl").unwrap_or(header);
            Some((header.split_whitespace().collect::<Vec<_>>().join(" "), rule.kind))
        }
        "lexical_declaration" | "variable_declaration" => {
            let declarator = node.named_child(0).filter(|_| node.named_child_count() == 1)?;
            let value = declarator.child_by_field_name("value")?;
            matches!(value.kind(), "arrow_function" | "function_expression" | "generator_function")
                .then(|| named(declarator))
                .flatten()
                .map(|name| (name, rule.kind))
        }
        "type_declaration" => {
            let spec = node.named_child(0)?;
            let kind = match spec.child_by_field_name("type").map(|shape| shape.kind()) {
                Some("struct_type") => EntityKind::Struct,
                Some("interface_type") => EntityKind::Interface,
                _ => EntityKind::TypeAlias,
            };
            named(spec).map(|name| (name, kind))
        }
        _ => named(node).map(|name| (name, rule.kind)),
    }
}

/// Start of the declaration including doc comments and attributes stacked directly above it.
pub fn attached_start(outer: Node<'_>, attributes: &[&str]) -> usize {
    let mut first = outer;
    while let Some(previous) = first.prev_sibling() {
        let is_comment = previous.kind().contains("comment");
        let trails_code =
            previous.prev_sibling().is_some_and(|before| before.end_position().row == previous.start_position().row);
        let detached = first.start_position().row.saturating_sub(previous.end_position().row) > 1;
        if !(is_comment || attributes.contains(&previous.kind())) || (is_comment && trails_code) || detached {
            break;
        }
        first = previous;
    }
    first.start_byte()
}

/// Whether a node `distance` levels below a declaration is that declaration's body.
/// The distance cap keeps bodies of lambdas in default arguments from being mistaken for it.
pub fn is_body(kind: EntityKind, field: Option<&str>, distance: usize) -> bool {
    use EntityKind::*;
    match field {
        Some("body") => distance <= 3,
        Some("value") => distance <= 2 && matches!(kind, Constant | TypeAlias),
        Some("type") => distance <= 2 && matches!(kind, TypeAlias | Struct | Interface),
        _ => false,
    }
}

/// Go methods are declared at file level; their receiver type acts as the container.
pub fn receiver_type(node: Node<'_>) -> Option<Node<'_>> {
    let mut pending = vec![node.child_by_field_name("receiver")?];
    while let Some(current) = pending.pop() {
        if current.kind() == "type_identifier" {
            return Some(current);
        }
        pending.extend((0..current.child_count()).rev().filter_map(|index| current.child(index)));
    }
    None
}

/// Extends a node span to whole lines when nothing else shares them.
pub fn widen_to_lines(source: &str, span: Range<usize>) -> Range<usize> {
    let bytes = source.as_bytes();
    let indent = bytes[..span.start].iter().rev().take_while(|byte| matches!(byte, b' ' | b'\t')).count();
    let line_start = span.start - indent;
    let start = if line_start == 0 || bytes[line_start - 1] == b'\n' { line_start } else { span.start };

    let trailing =
        bytes[span.end..].iter().take_while(|byte| matches!(byte, b' ' | b'\t' | b';' | b',' | b'\r')).count();
    let line_end = span.end + trailing;
    let end = match bytes.get(line_end) {
        Some(b'\n') => line_end + 1,
        None => line_end,
        Some(_) => span.end,
    };
    start..end
}
