import type { DiffLine } from "./types";

/** How a local branch relates to the remote branch it tracks. */
export interface Tracking {
  branch: string;
  upstream: string;
  ahead: number;
  behind: number;
  gone: boolean;
}

export type PullMode = "fast_forward" | "rebase" | "merge";
export type MergeMode = "auto" | "no_fast_forward" | "squash";
export type ResetMode = "soft" | "mixed" | "hard";
export type OperationStep = "continue" | "skip" | "abort";
export type ConflictSide = "ours" | "theirs";

/** Changed lines picked in the raw diff, by the numbers shown in its two gutters. */
export interface LineSelection {
  old: number[];
  new: number[];
}

export interface BlameHunk {
  /** All zeros for lines that are not committed yet. */
  oid: string;
  author: string;
  time: number;
  summary: string;
  start: number;
  count: number;
}

export interface Blame {
  hunks: BlameHunk[];
  lines: DiffLine[];
}
