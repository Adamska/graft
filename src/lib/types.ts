export type RefKind = "local" | "remote" | "tag";

export interface RefEntry {
  name: string;
  kind: RefKind;
  oid: string;
  is_head: boolean;
}

export interface Stash {
  index: number;
  message: string;
  oid: string;
}

export interface RepoSnapshot {
  path: string;
  name: string;
  head: { branch: string | null; oid: string | null };
  refs: RefEntry[];
  stashes: Stash[];
}

export interface RecentRepository {
  path: string;
  name: string;
  opened_at: number;
}

export type ChangeStatus = "added" | "modified" | "deleted" | "renamed" | "untracked" | "conflicted";

export interface FileChange {
  path: string;
  old_path: string | null;
  status: ChangeStatus;
}

export interface WorkingStatus {
  staged: FileChange[];
  unstaged: FileChange[];
}

export interface CommitDetails {
  oid: string;
  summary: string;
  body: string;
  author: string;
  email: string;
  time: number;
  parents: string[];
  files: FileChange[];
}

export interface GraphStats {
  commits: number;
  lanes: number;
  micros: number;
}

export interface EngineMetrics {
  resident_bytes: number;
  threads: number;
}

export type DiffTarget = { kind: "commit"; oid: string } | { kind: "staged" } | { kind: "unstaged" };

export type ChangeKind = "moved" | "moved_edited" | "signature" | "body" | "modified" | "added" | "deleted" | "lines" | "origin";

export type EntityKind =
  | "function"
  | "method"
  | "struct"
  | "enum"
  | "trait"
  | "impl"
  | "class"
  | "interface"
  | "type_alias"
  | "module"
  | "constant"
  | "macro";

export interface ChangeSummary {
  kind: ChangeKind;
  entity: EntityKind;
  name: string;
  similarity: number;
  old_line: number | null;
  new_line: number | null;
  other_path: string | null;
}

export interface FileSummary extends FileChange {
  language: string | null;
  structural: boolean;
  binary: boolean;
  changes: ChangeSummary[];
}

export interface ChangesetSummary {
  files: FileSummary[];
  micros: number;
}

/** `[text, syntax class, changed flag]` */
export type Segment = [text: string, tokenClass: number, changed: 0 | 1];

export interface DiffLine {
  number: number;
  segments: Segment[];
}

export type RowKind = "context" | "change" | "add" | "del" | "fold";

export interface DiffRow {
  kind: RowKind;
  old: DiffLine | null;
  new: DiffLine | null;
  hidden: number;
}

export interface Location {
  path: string | null;
  start: number;
  end: number;
}

export interface Block {
  id: number;
  kind: ChangeKind;
  entity: EntityKind | null;
  name: string;
  qualified: string;
  old: Location | null;
  new: Location | null;
  similarity: number;
  renamed_from: string | null;
  link: number | null;
  stageable: boolean;
  rows: DiffRow[];
}

export interface FileDiff {
  path: string;
  old_path: string | null;
  language: string | null;
  structural: boolean;
  binary: boolean;
  blocks: Block[];
}

export interface EntitySelector {
  qualified: string;
  old_start: number | null;
  new_start: number | null;
}

export type Severity = "high" | "medium" | "low";

export interface RiskFinding {
  path: string;
  symbol: string;
  severity: Severity;
  title: string;
  detail: string;
  source: string;
}

export interface RiskReport {
  findings: RiskFinding[];
  model: string | null;
  notice: string | null;
}

export interface CommitDraft {
  message: string;
  model: string | null;
  notice: string | null;
}

export type AssistantProvider = "anthropic" | "openai" | "ollama";

export interface AssistantStatus {
  provider: AssistantProvider | null;
  base_url: string | null;
  model: string | null;
  has_key: boolean;
  ready: boolean;
  detail: string;
}

export interface AssistantSettings {
  provider: AssistantProvider | null;
  base_url: string | null;
  model: string | null;
  api_key: string | null;
}
