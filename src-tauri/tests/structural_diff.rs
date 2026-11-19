use graft_lib::ast_diff::blocks::FileDiff;
use graft_lib::ast_diff::model::RowKind;
use graft_lib::ast_diff::render::render;
use graft_lib::ast_diff::{analyze, Analysis, ChangeKind, EntityKind, SourcePair};
use graft_lib::git::{ChangeStatus, FileChange};

fn pair(path: &str, old: &str, new: &str) -> SourcePair {
    SourcePair {
        change: FileChange { path: path.to_string(), old_path: None, status: ChangeStatus::Modified },
        old: Some(old.to_string()),
        new: Some(new.to_string()),
    }
}

/// `(kind, qualified name)` of every change, in the engine's stable order.
fn kinds(analysis: &Analysis) -> Vec<(ChangeKind, String)> {
    analysis
        .changes
        .iter()
        .map(|change| {
            let name = match (change.new, change.old) {
                (Some(at), _) => analysis.new_entity(at).qualified.clone(),
                (None, Some(at)) => analysis.old_entity(at).qualified.clone(),
                (None, None) => unreachable!("a change always has a side"),
            };
            (change.kind, name)
        })
        .collect()
}

fn diff(path: &str, old: &str, new: &str) -> (Analysis, FileDiff) {
    let analysis = analyze(vec![pair(path, old, new)]);
    let file = render(&analysis, 0);
    (analysis, file)
}

const RUST_OLD: &str = "\
use std::fmt;

/// Reads one byte.
pub fn read(input: &[u8]) -> Option<u8> {
    input.first().copied()
}

pub fn checksum(input: &[u8]) -> u32 {
    let mut sum = 0u32;
    for byte in input {
        sum = sum.wrapping_add(u32::from(*byte));
    }
    sum
}

pub fn is_empty(input: &[u8]) -> bool {
    input.is_empty()
}
";

#[test]
fn whitespace_and_comment_edits_are_not_changes() {
    let new = RUST_OLD
        .replace("/// Reads one byte.", "/// Reads a single byte from the front.")
        .replace("    let mut sum = 0u32;", "    let mut sum   =   0u32; // running total");
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert!(analysis.changes.is_empty(), "{:?}", kinds(&analysis));
    assert!(file.blocks.is_empty());
}

#[test]
fn relocated_function_is_reported_as_moved_and_nothing_else() {
    let moved = "pub fn read(input: &[u8]) -> Option<u8> {\n    input.first().copied()\n}\n";
    let new =
        format!("{}\n/// Reads one byte.\n{moved}", RUST_OLD.replace(&format!("/// Reads one byte.\n{moved}\n"), ""));
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert_eq!(kinds(&analysis), [(ChangeKind::Moved, "read".to_string())]);
    let kinds: Vec<_> = file.blocks.iter().map(|block| block.kind).collect();
    assert_eq!(kinds, [ChangeKind::Origin, ChangeKind::Moved]);
    assert_eq!(file.blocks[0].link, Some(1));
    assert_eq!(file.blocks[1].link, Some(0));
    assert_eq!(file.blocks[1].old.as_ref().map(|at| at.start), Some(3));
    assert_eq!(file.blocks[1].new.as_ref().map(|at| at.start), Some(15));
    assert!(file.blocks[1].rows.iter().all(|row| matches!(row.kind, RowKind::Context | RowKind::Fold)));
}

#[test]
fn signature_and_body_edits_are_told_apart() {
    let new = RUST_OLD
        .replace("pub fn read(input: &[u8]) -> Option<u8>", "pub fn read(input: &[u8], at: usize) -> Option<u8>")
        .replace("sum.wrapping_add(u32::from(*byte))", "sum.rotate_left(1) ^ u32::from(*byte)")
        .replace(
            "fn is_empty(input: &[u8]) -> bool {\n    input.is_empty()",
            "fn is_blank(input: &[u8]) -> bool {\n    input.iter().all(u8::is_ascii_whitespace)",
        );
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert_eq!(
        kinds(&analysis),
        [
            (ChangeKind::Signature, "read".to_string()),
            (ChangeKind::Body, "checksum".to_string()),
            (ChangeKind::Added, "is_blank".to_string()),
            (ChangeKind::Deleted, "is_empty".to_string()),
        ]
    );
    let body = file.blocks.iter().find(|block| block.name == "checksum").expect("checksum block");
    let changed: Vec<_> = body.rows.iter().filter(|row| row.kind == RowKind::Change).collect();
    assert_eq!(changed.len(), 1);
    let flagged: String = changed[0]
        .new
        .as_ref()
        .expect("new side")
        .segments
        .iter()
        .filter(|s| s.changed)
        .map(|s| s.text.as_str())
        .collect();
    assert!(flagged.contains("rotate_left"), "flagged tokens: {flagged:?}");
    assert!(!flagged.contains("sum ="), "unchanged tokens must stay unflagged: {flagged:?}");
}

#[test]
fn function_moved_to_another_file_and_edited_keeps_its_identity() {
    let lexer_new = RUST_OLD.replace(
        "pub fn checksum(input: &[u8]) -> u32 {\n    let mut sum = 0u32;\n    for byte in input {\n        sum = sum.wrapping_add(u32::from(*byte));\n    }\n    sum\n}\n\n",
        "",
    );
    let parser_new = "pub fn checksum(input: &[u8]) -> u32 {\n    let mut sum = 1u32;\n    for byte in input {\n        sum = sum.wrapping_add(u32::from(*byte));\n    }\n    sum\n}\n";
    let mut added = pair("src/sum.rs", "", parser_new);
    added.change.status = ChangeStatus::Added;
    let analysis = analyze(vec![pair("src/io.rs", RUST_OLD, &lexer_new), added]);

    assert_eq!(kinds(&analysis), [(ChangeKind::MovedEdited, "checksum".to_string())]);
    let similarity = analysis.changes[0].similarity;
    assert!((0.9..1.0).contains(&similarity), "similarity {similarity}");

    let summary = analysis.summary();
    assert_eq!(summary.files[0].changes[0].kind, ChangeKind::Origin);
    assert_eq!(summary.files[0].changes[0].other_path.as_deref(), Some("src/sum.rs"));
    assert_eq!(summary.files[1].changes[0].other_path.as_deref(), Some("src/io.rs"));

    let destination = render(&analysis, 1);
    assert_eq!(destination.blocks[0].old.as_ref().and_then(|at| at.path.as_deref()), Some("src/io.rs"));
    let origin = render(&analysis, 0);
    assert_eq!(origin.blocks[0].kind, ChangeKind::Origin);
    assert_eq!(origin.blocks[0].new.as_ref().and_then(|at| at.path.as_deref()), Some("src/sum.rs"));
}

const TS_OLD: &str = "\
import { request } from './http';

export class SessionStore {
  private cache = new Map<string, User>();

  async fetchUser(id: string): Promise<User> {
    const cached = this.cache.get(id);
    if (cached) return cached;
    const user = await request<User>(`/users/${id}`);
    this.cache.set(id, user);
    return user;
  }

  clear(): void {
    this.cache.clear();
  }
}

export const retry = async <T>(run: () => Promise<T>, attempts = 3): Promise<T> => {
  for (let attempt = 1; ; attempt++) {
    try {
      return await run();
    } catch (error) {
      if (attempt >= attempts) throw error;
    }
  }
};
";

#[test]
fn renamed_method_is_matched_inside_its_class() {
    let new = TS_OLD.replace("async fetchUser(id: string)", "async loadUser(id: string)");
    let (analysis, file) = diff("web/session.ts", TS_OLD, &new);

    assert_eq!(kinds(&analysis), [(ChangeKind::Signature, "SessionStore.loadUser".to_string())]);
    assert_eq!(file.blocks.len(), 1);
    assert_eq!(file.blocks[0].entity, Some(EntityKind::Method));
    assert_eq!(file.blocks[0].renamed_from.as_deref(), Some("fetchUser"));
}

#[test]
fn exported_arrow_function_is_a_declaration_and_class_fields_are_shallow() {
    let new = TS_OLD
        .replace("private cache = new Map<string, User>();", "private readonly cache = new Map<string, User>();")
        .replace("attempts = 3", "attempts = 5");
    let (analysis, _) = diff("web/session.ts", TS_OLD, &new);

    assert_eq!(
        kinds(&analysis),
        [(ChangeKind::Body, "SessionStore".to_string()), (ChangeKind::Signature, "retry".to_string())]
    );
    let retry = analysis.changes[1].new.map(|at| analysis.new_entity(at)).expect("retry entity");
    assert_eq!(retry.kind, EntityKind::Function);
    assert_eq!(retry.lines, 18..27, "the span covers the export keyword through the closing brace");
}

#[test]
fn python_nesting_change_is_structural_even_when_tokens_match() {
    let old = "def drain(queue):\n    for item in queue:\n        if item.ready:\n            item.run()\n            item.done()\n";
    let new = "def drain(queue):\n    for item in queue:\n        if item.ready:\n            item.run()\n        item.done()\n";
    let (analysis, _) = diff("tools/drain.py", old, new);

    assert_eq!(kinds(&analysis), [(ChangeKind::Body, "drain".to_string())]);
}

#[test]
fn python_decorated_method_moves_between_classes() {
    let old = "class Report:\n    @staticmethod\n    def percentile(values, rank):\n        ordered = sorted(values)\n        return ordered[int(len(ordered) * rank)]\n\n    def render(self):\n        return str(self)\n\n\nclass Stats:\n    def mean(self, values):\n        return sum(values) / len(values)\n";
    let new = "class Report:\n    def render(self):\n        return str(self)\n\n\nclass Stats:\n    def mean(self, values):\n        return sum(values) / len(values)\n\n    @staticmethod\n    def percentile(values, rank):\n        ordered = sorted(values)\n        return ordered[int(len(ordered) * rank)]\n";
    let (analysis, _) = diff("tools/report.py", old, new);

    assert_eq!(kinds(&analysis), [(ChangeKind::Moved, "Stats.percentile".to_string())]);
    let moved = analysis.changes[0].new.map(|at| analysis.new_entity(at)).expect("moved entity");
    assert_eq!(moved.lines, 9..13, "the decorator travels with the method");
}

#[test]
fn go_methods_are_qualified_by_receiver_and_types_by_shape() {
    let old = "package relay\n\ntype Server struct {\n\taddr string\n}\n\nfunc (s *Server) Listen() error {\n\treturn listen(s.addr)\n}\n\nfunc listen(addr string) error {\n\treturn nil\n}\n";
    let new = "package relay\n\ntype Server struct {\n\taddr    string\n\ttimeout int\n}\n\nfunc listen(addr string) error {\n\treturn nil\n}\n\nfunc (s *Server) Listen() error {\n\treturn listen(s.addr)\n}\n";
    let (analysis, _) = diff("cmd/relay/main.go", old, new);

    let found = kinds(&analysis);
    assert_eq!(found[0], (ChangeKind::Body, "Server".to_string()));
    assert_eq!(found.len(), 2);
    assert_eq!(found[1].0, ChangeKind::Moved);
    assert!(["listen", "Server.Listen"].contains(&found[1].1.as_str()), "{found:?}");
    let server = analysis.changes[0].new.map(|at| analysis.new_entity(at)).expect("server entity");
    assert_eq!(server.kind, EntityKind::Struct);
}

#[test]
fn unsupported_formats_fall_back_to_line_hunks_with_word_highlights() {
    let old = "# Relay\n\nA small message relay.\n\n## Usage\n\nRun `relay --port 4000`.\n";
    let new = "# Relay\n\nA small message relay.\n\n## Usage\n\nRun `relay --port 4100`.\n\n## License\n\nMIT\n";
    let (analysis, file) = diff("README.md", old, new);

    assert!(analysis.changes.is_empty());
    assert!(!file.structural);
    assert_eq!(file.blocks.len(), 1);
    assert_eq!(file.blocks[0].kind, ChangeKind::Lines);
    let change = file.blocks[0].rows.iter().find(|row| row.kind == RowKind::Change).expect("changed line");
    let flagged: String =
        change.new.as_ref().expect("new side").segments.iter().filter(|s| s.changed).map(|s| s.text.as_str()).collect();
    assert_eq!(flagged, "4100");
    assert_eq!(file.blocks[0].rows.iter().filter(|row| row.kind == RowKind::Add).count(), 4);
}

#[test]
fn top_level_code_outside_declarations_is_reported_as_line_hunks() {
    let new = RUST_OLD.replace("use std::fmt;", "use std::fmt;\nuse std::io;");
    let (analysis, file) = diff("src/io.rs", RUST_OLD, &new);

    assert!(analysis.changes.is_empty());
    assert_eq!(file.blocks.len(), 1);
    assert_eq!(file.blocks[0].kind, ChangeKind::Lines);
    assert_eq!(file.blocks[0].new.as_ref().map(|at| (at.start, at.end)), Some((1, 3)));
}
