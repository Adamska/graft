use super::model::{ParsedFile, Token, TokenClass};

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Clone, Copy)]
pub struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Self(FNV_OFFSET)
    }
}

impl Fnv {
    pub fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(FNV_PRIME);
        }
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

pub fn text_hash(text: &str) -> u64 {
    let mut hasher = Fnv::default();
    hasher.write(text.as_bytes());
    hasher.finish()
}

/// Whitespace- and comment-insensitive fingerprint of a token run.
pub fn structural_hash<'a>(file: &ParsedFile, tokens: impl IntoIterator<Item = &'a Token>, base_depth: u16) -> u64 {
    let mut hasher = Fnv::default();
    for token in tokens {
        if token.class == TokenClass::Comment {
            continue;
        }
        hasher.write(&token.kind.to_le_bytes());
        hasher.write(&token.depth.wrapping_sub(base_depth).to_le_bytes());
        hasher.write(file.token_text(token).as_bytes());
        hasher.write(&[0xff]);
    }
    hasher.finish()
}

/// Comment and string nodes are kept whole instead of being split into their fragments.
pub fn is_atomic(kind: &str, named: bool) -> bool {
    kind.contains("comment") || (named && is_string_like(kind))
}

fn is_string_like(kind: &str) -> bool {
    (kind.contains("string") && !kind.contains("string_content"))
        || kind.contains("char_literal")
        || kind == "rune_literal"
        || kind == "regex"
}

pub struct Context<'a> {
    pub kind: &'a str,
    pub named: bool,
    pub field: Option<&'a str>,
    pub parent_kind: &'a str,
    pub parent_field: Option<&'a str>,
}

pub fn classify(context: &Context<'_>, text: &str) -> TokenClass {
    let Context { kind, named, field, parent_kind, parent_field } = *context;
    if kind.contains("comment") {
        return TokenClass::Comment;
    }
    if named && is_string_like(kind) {
        return TokenClass::String;
    }
    if kind.contains("integer") || kind.contains("float") || kind.contains("number") || kind == "imaginary_literal" {
        return TokenClass::Number;
    }
    if matches!(kind, "true" | "false" | "null" | "nil" | "none" | "None" | "undefined" | "boolean_literal" | "iota") {
        return TokenClass::Constant;
    }
    if matches!(kind, "type_identifier" | "primitive_type" | "predefined_type")
        || matches!(parent_kind, "primitive_type" | "predefined_type")
    {
        return TokenClass::Type;
    }
    if matches!(kind, "self" | "super" | "crate" | "this" | "mutable_specifier") {
        return TokenClass::Keyword;
    }
    if kind.ends_with("identifier") {
        let declares = field == Some("name") && (parent_kind.contains("function") || parent_kind.contains("method"));
        let called = matches!(field, Some("function" | "macro"))
            || (matches!(field, Some("field" | "property" | "attribute" | "name")) && parent_field == Some("function"));
        if declares || called {
            return TokenClass::Function;
        }
        if matches!(kind, "field_identifier" | "property_identifier" | "shorthand_property_identifier") {
            return TokenClass::Property;
        }
        return TokenClass::Plain;
    }
    if named {
        return TokenClass::Plain;
    }
    if text.starts_with(|first: char| first.is_alphabetic()) {
        TokenClass::Keyword
    } else {
        TokenClass::Punctuation
    }
}

/// Word-level tokenizer for formats without a grammar: identifiers, numbers, and single symbols.
pub fn lex_plain(file: &mut ParsedFile) {
    let bytes = file.source.as_bytes();
    let mut at = 0;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte.is_ascii_whitespace() {
            at += 1;
            continue;
        }
        let start = at;
        let class = if is_word(byte) {
            while at < bytes.len() && is_word(bytes[at]) {
                at += 1;
            }
            if byte.is_ascii_digit() {
                TokenClass::Number
            } else {
                TokenClass::Plain
            }
        } else {
            at += 1;
            TokenClass::Punctuation
        };
        file.tokens.push(Token { start: start as u32, end: at as u32, class, kind: 0, depth: 0 });
    }
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte >= 0x80
}
