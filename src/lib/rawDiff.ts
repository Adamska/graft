import type { LineSelection } from "./actionTypes";
import type { DiffLine, DiffRow } from "./types";

export interface Hunk {
  /** Index of the hunk's header in the unified line list. */
  line: number;
  oldStart: number;
  oldCount: number;
  newStart: number;
  newCount: number;
  /** Every changed line of the hunk: what staging the whole hunk selects. */
  changes: LineSelection;
}

export type RawLine =
  | { kind: "hunk"; hunk: number }
  | { kind: "context" | "add" | "del"; hunk: number; old: number | null; new: number | null; line: DiffLine };

export interface RawDiff {
  lines: RawLine[];
  hunks: Hunk[];
}

/**
 * Turns aligned rows into a unified diff. Folds become hunk boundaries, runs of context
 * that border no change are dropped, and within a run of changed pairs the deletions are
 * listed before the additions.
 */
export function unify(rows: DiffRow[]): RawDiff {
  const groups: { hunk: Hunk; body: RawLine[] }[] = [];
  let additions: RawLine[] = [];
  let open: { hunk: Hunk; body: RawLine[] } | null = null;

  const flush = () => {
    open?.body.push(...additions);
    additions = [];
  };
  for (const row of rows) {
    if (row.kind !== "change") flush();
    if (row.kind === "fold") {
      open = null;
      continue;
    }
    if (!open) {
      open = { hunk: { line: 0, oldStart: 0, oldCount: 0, newStart: 0, newCount: 0, changes: { old: [], new: [] } }, body: [] };
      groups.push(open);
    }
    const { hunk, body } = open;
    if (row.old) {
      hunk.oldStart ||= row.old.number;
      hunk.oldCount++;
    }
    if (row.new) {
      hunk.newStart ||= row.new.number;
      hunk.newCount++;
    }
    if (row.kind === "context" && row.new) {
      body.push({ kind: "context", hunk: 0, old: row.old?.number ?? null, new: row.new.number, line: row.new });
      continue;
    }
    if (row.old) {
      hunk.changes.old.push(row.old.number);
      body.push({ kind: "del", hunk: 0, old: row.old.number, new: null, line: row.old });
    }
    if (row.new) {
      hunk.changes.new.push(row.new.number);
      additions.push({ kind: "add", hunk: 0, old: null, new: row.new.number, line: row.new });
    }
  }
  flush();

  const lines: RawLine[] = [];
  const hunks: Hunk[] = [];
  for (const { hunk, body } of groups) {
    if (selectionSize(hunk.changes) === 0) continue;
    const index = hunks.length;
    hunk.line = lines.length;
    hunks.push(hunk);
    lines.push({ kind: "hunk", hunk: index });
    for (const line of body) lines.push({ ...line, hunk: index });
  }
  return { lines, hunks };
}

/** `@@ -12,7 +12,9 @@`, as git would print the hunk's extent. */
export function hunkLabel(hunk: Hunk): string {
  return `@@ -${hunk.oldStart},${hunk.oldCount} +${hunk.newStart},${hunk.newCount} @@`;
}

export function selectionSize(selection: LineSelection): number {
  return selection.old.length + selection.new.length;
}
