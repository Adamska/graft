use super::recipes::Version::{self, *};

pub const AUTHORS: [(&str, &str); 3] =
    [("Ada Park", "ada@relay.dev"), ("Jon Reyes", "jon@relay.dev"), ("Mira Castell", "mira@relay.dev")];
const ADA: usize = 0;
const JON: usize = 1;
const MIRA: usize = 2;

const MAIN: &str = "main";
const PARSER: &str = "feature/parser-split";
const SESSION: &str = "refactor/session-api";
const HOTFIX: &str = "hotfix/varint-overflow";
const ROTATION: &str = "feature/token-rotation";

type Files = &'static [(&'static str, Version)];

pub struct Commit {
    pub label: &'static str,
    pub branch: &'static str,
    pub parents: &'static [&'static str],
    pub author: usize,
    pub message: &'static str,
    /// Files that differ from the first parent's snapshot.
    pub files: Files,
}

const fn commit(
    label: &'static str,
    branch: &'static str,
    parents: &'static [&'static str],
    author: usize,
    message: &'static str,
    files: Files,
) -> Commit {
    Commit { label, branch, parents, author, message, files }
}

pub const TAGS: [(&str, &str); 2] = [("c5", "v0.1.0"), ("c8", "v0.2.0")];

/// Topic branches that also exist on the sandbox remote.
pub const PUBLISHED: [&str; 2] = [ROTATION, HOTFIX];

/// Uncommitted work left in the sandbox: `(path, revision, staged)`.
pub const PENDING: [(&str, Version, bool); 4] = [
    ("web/api.ts", ApiTimeout, true),
    ("src/auth.rs", AuthPending, false),
    ("config/relay.yaml", ConfigShortTtl, false),
    ("notes/todo.md", Todo, false),
];

const BOOTSTRAP: Files =
    &[("README.md", Readme), ("config/relay.yaml", Config), ("src/lexer.rs", Lexer), ("src/auth.rs", Auth)];
const SPLIT: Files = &[("src/lexer.rs", LexerSplit), ("src/parser.rs", ParserLimited)];
const SPLIT_MERGED: Files = &[("src/lexer.rs", LexerSplit), ("src/parser.rs", ParserChecked)];
const ROTATE: Files = &[("src/auth.rs", AuthRotation), ("config/relay.yaml", ConfigRotation)];
const SCANNER: Files = &[("src/lexer.rs", LexerScanner), ("src/parser.rs", ParserDelimiter)];

/// The scripted history, oldest first. Three topic branches fork and merge back, one
/// stays ahead of `main`, and each commit showcases a different structural change.
pub const SCRIPT: [Commit; 20] = [
    commit("c1", MAIN, &[], ADA, "chore: bootstrap relay workspace", BOOTSTRAP),
    commit(
        "c2",
        MAIN,
        &["c1"],
        MIRA,
        "feat(web): add session store and api client",
        &[("web/session.ts", Session), ("web/api.ts", Api)],
    ),
    commit("c3", MAIN, &["c2"], JON, "feat(tools): add latency report script", &[("tools/report.py", Report)]),
    commit("c4", MAIN, &["c3"], JON, "feat(cmd): add relay entrypoint", &[("cmd/relay/main.go", Relay)]),
    commit("p1", PARSER, &["c4"], ADA, "refactor(parser): move frame and header decoding out of the lexer", SPLIT),
    commit(
        "c5",
        MAIN,
        &["c4"],
        ADA,
        "fix(auth): treat tokens without expiry as expired",
        &[("src/auth.rs", AuthStrict)],
    ),
    commit(
        "p2",
        PARSER,
        &["p1"],
        ADA,
        "feat(parser): report oversized and truncated frames",
        &[("src/parser.rs", ParserChecked)],
    ),
    commit(
        "s1",
        SESSION,
        &["c5"],
        MIRA,
        "refactor(web): rename fetchUser to loadUser and hoist retry",
        &[("web/session.ts", SessionRenamed)],
    ),
    commit(
        "h1",
        HOTFIX,
        &["c5"],
        JON,
        "fix(lexer): stop decoding varints after ten bytes",
        &[("src/lexer.rs", LexerBounded)],
    ),
    commit("m1", MAIN, &["c5", "p2"], ADA, "Merge branch 'feature/parser-split'", SPLIT_MERGED),
    commit(
        "s2",
        SESSION,
        &["s1"],
        MIRA,
        "refactor(web): group session helpers above the store",
        &[("web/session.ts", SessionGrouped)],
    ),
    commit(
        "c6",
        MAIN,
        &["m1"],
        JON,
        "perf(tools): sort samples once per report",
        &[("tools/report.py", ReportPresorted)],
    ),
    commit(
        "m2",
        MAIN,
        &["c6", "h1"],
        JON,
        "Merge branch 'hotfix/varint-overflow'",
        &[("src/lexer.rs", LexerSplitBounded)],
    ),
    commit(
        "m3",
        MAIN,
        &["m2", "s2"],
        MIRA,
        "Merge branch 'refactor/session-api'",
        &[("web/session.ts", SessionGrouped)],
    ),
    commit(
        "c7",
        MAIN,
        &["m3"],
        JON,
        "feat(cmd): shut down gracefully and extract listener setup",
        &[("cmd/relay/main.go", RelayGraceful)],
    ),
    commit("r1", ROTATION, &["c7"], ADA, "feat(auth): rotate tokens ahead of expiry", ROTATE),
    commit(
        "c8",
        MAIN,
        &["c7"],
        MIRA,
        "docs: describe the frame format and token lifecycle",
        &[("README.md", ReadmeFormats)],
    ),
    commit(
        "r2",
        ROTATION,
        &["r1"],
        ADA,
        "test(auth): cover rotation window edge cases",
        &[("tests/auth_rotation.rs", RotationTest)],
    ),
    commit(
        "r3",
        ROTATION,
        &["r2", "c8"],
        ADA,
        "Merge branch 'main' into feature/token-rotation",
        &[("README.md", ReadmeFormats)],
    ),
    commit("c9", MAIN, &["c8"], ADA, "refactor(lexer): rename is_separator and add token scanner", SCANNER),
];
