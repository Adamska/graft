use super::fixtures::{join, AUTH, DOCS, RELAY, REPORT, WEB, WIRE};
use graft_lib::error::Result;

/// Every distinct file revision that appears in the sandbox history or working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    Readme,
    ReadmeFormats,
    Config,
    ConfigRotation,
    ConfigShortTtl,
    Todo,
    Lexer,
    LexerBounded,
    LexerSplit,
    LexerSplitBounded,
    LexerScanner,
    ParserLimited,
    ParserChecked,
    ParserDelimiter,
    Auth,
    AuthStrict,
    AuthRotation,
    AuthPending,
    RotationTest,
    Session,
    SessionRenamed,
    SessionGrouped,
    Api,
    ApiTimeout,
    Report,
    ReportPresorted,
    Relay,
    RelayGraceful,
}

impl Version {
    /// Assembles the revision from named fragments; reordering names is what moves code.
    pub fn render(self) -> Result<String> {
        use Version::*;
        match self {
            Readme => DOCS.file(&["readme"], 1),
            ReadmeFormats => DOCS.file(&["readme.formats"], 1),
            Config => DOCS.file(&["config"], 1),
            ConfigRotation => DOCS.file(&["config.rotation"], 1),
            ConfigShortTtl => DOCS.file(&["config.short"], 1),
            Todo => DOCS.file(&["todo"], 1),
            Lexer => lexer(&["types", "is_separator", "read_varint", "parse_header", "parse_frame", "skip_whitespace"]),
            LexerBounded => lexer(&[
                "types",
                "is_separator",
                "read_varint.bounded",
                "parse_header",
                "parse_frame",
                "skip_whitespace",
            ]),
            LexerSplit => lexer(&["is_separator", "read_varint", "skip_whitespace"]),
            LexerSplitBounded => lexer(&["is_separator", "read_varint.bounded", "skip_whitespace"]),
            LexerScanner => lexer(&["is_delimiter", "read_varint.bounded", "skip_whitespace", "scan_tokens"]),
            ParserLimited => WIRE.file(&["parser.prelude", "types", "parse_header", "parse_frame.limited"], 1),
            ParserChecked => {
                WIRE.file(&["parser.prelude", "types", "parse_error", "parse_header", "parse_frame.checked"], 1)
            }
            ParserDelimiter => WIRE.file(
                &["parser.prelude.delimiter", "types", "parse_error", "parse_header.delimiter", "parse_frame.checked"],
                1,
            ),
            Auth => auth("error", &["is_expired", "validate", "refresh", "revoke"]),
            AuthStrict => auth("error.subjects", &["is_expired.strict", "validate.known", "refresh", "revoke"]),
            AuthRotation => auth(
                "error.subjects",
                &["is_expired.strict", "validate.known", "refresh", "revoke", "needs_rotation", "rotate"],
            ),
            AuthPending => auth(
                "error.subjects",
                &["is_expired.inclusive", "refresh", "revoke", "validate.relaxed", "remaining_ttl"],
            ),
            RotationTest => AUTH.file(&["rotation_test"], 1),
            Session => session(&[], &["fetch_user", "has_scope", "evict"], &["retry", "is_fresh"]),
            SessionRenamed => session(&["retry"], &["load_user", "has_scope", "evict"], &["is_fresh"]),
            SessionGrouped => session(&["is_fresh", "retry"], &["load_user", "evict", "has_scope"], &[]),
            Api => WEB.file(&["api.base", "request", "post"], 1),
            ApiTimeout => WEB.file(&["api.base", "request.timeout", "post"], 1),
            Report => REPORT.file(&["imports", "percentile", "load", "summarize", "main"], 2),
            ReportPresorted => REPORT.file(&["imports", "load", "summarize.sorted", "percentile.presorted", "main"], 2),
            Relay => RELAY.file(&["header", "server", "main", "serve", "handle"], 1),
            RelayGraceful => RELAY.file(&["header.signals", "server", "listen", "serve", "handle", "main.graceful"], 1),
        }
    }
}

fn lexer(parts: &[&str]) -> Result<String> {
    let mut names = vec!["lexer.prelude"];
    names.extend_from_slice(parts);
    WIRE.file(&names, 1)
}

fn auth(error: &str, methods: &[&str]) -> Result<String> {
    let methods = AUTH.scope("impl AuthToken {", methods, "}")?;
    Ok(join(&[AUTH.part("prelude")?, AUTH.part("token")?, AUTH.part(error)?, &methods], 1))
}

fn session(before: &[&str], methods: &[&str], after: &[&str]) -> Result<String> {
    let store = WEB.scope(&format!("{}\n", WEB.part("store.open")?), methods, "}")?;
    let mut parts = vec![WEB.part("session.imports")?];
    for name in before {
        parts.push(WEB.part(name)?);
    }
    parts.push(&store);
    for name in after {
        parts.push(WEB.part(name)?);
    }
    Ok(join(&parts, 1))
}
