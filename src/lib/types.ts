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
