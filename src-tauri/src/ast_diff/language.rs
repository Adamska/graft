use tree_sitter::Language;

use super::model::EntityKind::{self, *};

pub struct EntityRule {
    pub node: &'static str,
    pub kind: EntityKind,
    pub container: bool,
}

pub struct LanguageSpec {
    pub name: &'static str,
    pub grammar: fn() -> Language,
    pub entities: &'static [EntityRule],
    /// Nodes the extractor looks through when searching for nested declarations.
    pub transparent: &'static [&'static str],
    /// Nodes whose span a wrapped declaration adopts, such as `export` or decorators.
    pub wrappers: &'static [&'static str],
    /// Preceding siblings that belong to a declaration besides comments.
    pub attributes: &'static [&'static str],
    pub separator: &'static str,
}

impl LanguageSpec {
    pub fn rule(&self, node: &str) -> Option<&EntityRule> {
        self.entities.iter().find(|rule| rule.node == node)
    }
}

const fn leaf(node: &'static str, kind: EntityKind) -> EntityRule {
    EntityRule { node, kind, container: false }
}

const fn container(node: &'static str, kind: EntityKind) -> EntityRule {
    EntityRule { node, kind, container: true }
}

static RUST: LanguageSpec = LanguageSpec {
    name: "Rust",
    grammar: || tree_sitter_rust::LANGUAGE.into(),
    entities: &[
        leaf("function_item", Function),
        leaf("function_signature_item", Function),
        leaf("struct_item", Struct),
        leaf("union_item", Struct),
        leaf("enum_item", Enum),
        leaf("type_item", TypeAlias),
        leaf("const_item", Constant),
        leaf("static_item", Constant),
        leaf("macro_definition", Macro),
        container("trait_item", Trait),
        container("impl_item", Impl),
        container("mod_item", Module),
    ],
    transparent: &["source_file", "declaration_list"],
    wrappers: &[],
    attributes: &["attribute_item"],
    separator: "::",
};

const SCRIPT_ENTITIES: &[EntityRule] = &[
    leaf("function_declaration", Function),
    leaf("generator_function_declaration", Function),
    leaf("function_signature", Function),
    leaf("lexical_declaration", Function),
    leaf("variable_declaration", Function),
    leaf("method_definition", Method),
    leaf("abstract_method_signature", Method),
    leaf("interface_declaration", Interface),
    leaf("type_alias_declaration", TypeAlias),
    leaf("enum_declaration", Enum),
    container("class_declaration", Class),
    container("abstract_class_declaration", Class),
    container("internal_module", Module),
    container("module", Module),
];

const SCRIPT_TRANSPARENT: &[&str] =
    &["program", "export_statement", "ambient_declaration", "class_body", "statement_block"];
const SCRIPT_WRAPPERS: &[&str] = &["export_statement", "ambient_declaration"];

static TYPESCRIPT: LanguageSpec = LanguageSpec {
    name: "TypeScript",
    grammar: || tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
    entities: SCRIPT_ENTITIES,
    transparent: SCRIPT_TRANSPARENT,
    wrappers: SCRIPT_WRAPPERS,
    attributes: &["decorator"],
    separator: ".",
};

static TSX: LanguageSpec = LanguageSpec {
    name: "TSX",
    grammar: || tree_sitter_typescript::LANGUAGE_TSX.into(),
    entities: SCRIPT_ENTITIES,
    transparent: SCRIPT_TRANSPARENT,
    wrappers: SCRIPT_WRAPPERS,
    attributes: &["decorator"],
    separator: ".",
};

static JAVASCRIPT: LanguageSpec = LanguageSpec {
    name: "JavaScript",
    grammar: || tree_sitter_javascript::LANGUAGE.into(),
    entities: SCRIPT_ENTITIES,
    transparent: SCRIPT_TRANSPARENT,
    wrappers: SCRIPT_WRAPPERS,
    attributes: &["decorator"],
    separator: ".",
};

static PYTHON: LanguageSpec = LanguageSpec {
    name: "Python",
    grammar: || tree_sitter_python::LANGUAGE.into(),
    entities: &[leaf("function_definition", Function), container("class_definition", Class)],
    transparent: &["module", "decorated_definition", "block"],
    wrappers: &["decorated_definition"],
    attributes: &[],
    separator: ".",
};

static GO: LanguageSpec = LanguageSpec {
    name: "Go",
    grammar: || tree_sitter_go::LANGUAGE.into(),
    entities: &[
        leaf("function_declaration", Function),
        leaf("method_declaration", Method),
        leaf("type_declaration", TypeAlias),
    ],
    transparent: &["source_file"],
    wrappers: &[],
    attributes: &[],
    separator: ".",
};

pub fn for_path(path: &str) -> Option<&'static LanguageSpec> {
    let extension = path.rsplit_once('.')?.1.to_ascii_lowercase();
    Some(match extension.as_str() {
        "rs" => &RUST,
        "ts" | "mts" | "cts" => &TYPESCRIPT,
        "tsx" => &TSX,
        "js" | "mjs" | "cjs" | "jsx" => &JAVASCRIPT,
        "py" | "pyi" => &PYTHON,
        "go" => &GO,
        _ => return None,
    })
}
