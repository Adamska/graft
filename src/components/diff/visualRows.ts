import type { Block, DiffRow } from "../../lib/types";

export const ROW_HEIGHTS = { header: 30, code: 20, fold: 22, origin: 26, gap: 14 } as const;

export type VisualRow =
  | { kind: "header"; block: Block; collapsed: boolean }
  | { kind: "code"; block: Block; row: DiffRow }
  | { kind: "fold"; block: Block; hidden: number }
  | { kind: "origin"; block: Block }
  | { kind: "gap" };

/** Connects the marker at a declaration's former position with the block at its new one. */
export interface Ribbon {
  block: number;
  edited: boolean;
  fromTop: number;
  fromBottom: number;
  toTop: number;
  toBottom: number;
  /** Bottom edge of the destination block, for the accent bar along its extent. */
  blockBottom: number;
}

export interface DiffLayout {
  rows: VisualRow[];
  /** `offsets[i]` is the top of row `i`; the final entry is the total height. */
  offsets: Float64Array;
  ribbons: Ribbon[];
  headerRow: Map<number, number>;
}

export function isCollapsible(block: Block): boolean {
  return block.kind === "moved";
}

/** Flattens blocks into fixed-height rows so the viewer can window them by offset. */
export function buildLayout(blocks: Block[], expanded: ReadonlySet<number>): DiffLayout {
  const rows: VisualRow[] = [];
  const headerRow = new Map<number, number>();
  const lastRow = new Map<number, number>();

  for (const block of blocks) {
    if (rows.length > 0) rows.push({ kind: "gap" });
    headerRow.set(block.id, rows.length);
    if (block.kind === "origin") {
      rows.push({ kind: "origin", block });
    } else {
      const collapsed = isCollapsible(block) && !expanded.has(block.id);
      rows.push({ kind: "header", block, collapsed });
      if (!collapsed) {
        for (const row of block.rows) {
          rows.push(row.kind === "fold" ? { kind: "fold", block, hidden: row.hidden } : { kind: "code", block, row });
        }
      }
    }
    lastRow.set(block.id, rows.length - 1);
  }

  const offsets = new Float64Array(rows.length + 1);
  rows.forEach((row, index) => {
    offsets[index + 1] = (offsets[index] ?? 0) + ROW_HEIGHTS[row.kind];
  });
  const top = (index: number) => offsets[index] ?? 0;

  const ribbons: Ribbon[] = [];
  for (const block of blocks) {
    const origin = block.link === null ? undefined : headerRow.get(block.link);
    const header = headerRow.get(block.id);
    if (origin === undefined || header === undefined || (block.kind !== "moved" && block.kind !== "moved_edited")) continue;
    ribbons.push({
      block: block.id,
      edited: block.kind === "moved_edited",
      fromTop: top(origin),
      fromBottom: top(origin + 1),
      toTop: top(header),
      toBottom: top(header + 1),
      blockBottom: top((lastRow.get(block.id) ?? header) + 1),
    });
  }
  return { rows, offsets, ribbons, headerRow };
}

/** Index of the row containing vertical position `y`. */
export function rowAt(offsets: Float64Array, y: number): number {
  let [low, high] = [0, Math.max(0, offsets.length - 2)];
  while (low < high) {
    const middle = (low + high + 1) >> 1;
    if ((offsets[middle] ?? 0) <= y) low = middle;
    else high = middle - 1;
  }
  return low;
}
