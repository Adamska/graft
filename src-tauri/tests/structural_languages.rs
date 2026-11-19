mod common;

use common::{diff, kinds};
use graft_lib::ast_diff::model::RowKind;
use graft_lib::ast_diff::{ChangeKind, EntityKind};

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
