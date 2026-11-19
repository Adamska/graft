import { create } from "zustand";
import type { Blame } from "../lib/actionTypes";
import { errorMessage, ipc } from "../lib/ipc";
import { type RawDiff, unify } from "../lib/rawDiff";
import type { Block, ChangesetSummary, DiffTarget, FileDiff } from "../lib/types";
import { useRepoStore } from "./useRepoStore";
import { useUiStore } from "./useUiStore";

export type DiffMode = "structural" | "raw" | "blame";
export const DIFF_MODES: DiffMode[] = ["structural", "raw", "blame"];
type Summaries = Record<DiffTarget["kind"], ChangesetSummary | null>;

interface DiffState {
  target: DiffTarget | null;
  path: string | null;
  summaries: Summaries;
  file: FileDiff | null;
  raw: RawDiff | null;
  blame: Blame | null;
  mode: DiffMode;
  /** When off, declarations that only moved are hidden along with their origin markers. */
  showMoved: boolean;
  expanded: ReadonlySet<number>;
  activeBlock: number;
  /** Hunk the keyboard acts on in the raw view. */
  activeHunk: number;
  astMicros: number | null;

  sync: () => Promise<void>;
  open: (target: DiffTarget, path: string) => Promise<void>;
  setMode: (mode: DiffMode) => void;
  toggleMoved: () => void;
  toggleExpanded: (block: number) => void;
  setActiveBlock: (block: number) => void;
  setActiveHunk: (hunk: number) => void;
  stepBlock: (delta: number) => void;
}

const EMPTY: Summaries = { commit: null, staged: null, unstaged: null };
let generation = 0;

const report = (error: unknown) => useUiStore.getState().notify("error", errorMessage(error));
const sameTarget = (left: DiffTarget | null, right: DiffTarget) =>
  left?.kind === right.kind && (left.kind !== "commit" || (right.kind === "commit" && left.oid === right.oid));
const clamp = (value: number, count: number) => Math.max(0, Math.min(count - 1, value));

/** Blocks the user can step through and act on, honoring the moved-blocks toggle. */
export function visibleBlocks(file: FileDiff | null, showMoved: boolean): Block[] {
  if (!file) return [];
  return showMoved ? file.blocks : file.blocks.filter((block) => block.kind !== "moved" && block.kind !== "origin");
}

/** Opens on the first file with declaration-level changes, falling back to the first file. */
function firstInteresting(summary: ChangesetSummary): string | undefined {
  return (summary.files.find((file) => file.changes.length > 0) ?? summary.files[0])?.path;
}

/** The raw and blame views are loaded on demand; the structural diff always is. */
async function loadView(mode: DiffMode, target: DiffTarget, path: string): Promise<Pick<DiffState, "raw" | "blame">> {
  if (mode === "raw") return { raw: unify(await ipc.rawDiff(target, path)), blame: null };
  if (mode === "blame") return { raw: null, blame: await ipc.blameFile(target, path).catch(() => ({ hunks: [], lines: [] })) };
  return { raw: null, blame: null };
}

export const useDiffStore = create<DiffState>((set, get) => ({
  target: null,
  path: null,
  summaries: EMPTY,
  file: null,
  raw: null,
  blame: null,
  mode: "structural",
  showMoved: true,
  expanded: new Set(),
  activeBlock: -1,
  activeHunk: 0,
  astMicros: null,

  /** Re-analyzes whatever the repository selection points at and keeps the open file if it survived. */
  sync: async () => {
    const token = ++generation;
    const selection = useRepoStore.getState().selection;
    const { target, path } = get();
    try {
      if (selection.kind === "commit") {
        const commitTarget: DiffTarget = { kind: "commit", oid: selection.oid };
        const summary = await ipc.analyzeChangeset(commitTarget);
        if (token !== generation) return;
        set({ summaries: { ...EMPTY, commit: summary }, astMicros: summary.micros });
        const keep = sameTarget(target, commitTarget) && summary.files.some((file) => file.path === path);
        const next = keep ? path : firstInteresting(summary);
        if (next) return await get().open(commitTarget, next);
      } else {
        const [staged, unstaged] = await Promise.all([ipc.analyzeChangeset({ kind: "staged" }), ipc.analyzeChangeset({ kind: "unstaged" })]);
        if (token !== generation) return;
        set({ summaries: { commit: null, staged, unstaged }, astMicros: unstaged.micros + staged.micros });
        const lists = { staged, unstaged };
        const kind = target?.kind === "staged" || target?.kind === "unstaged" ? target.kind : null;
        if (kind && lists[kind].files.some((file) => file.path === path) && path) return await get().open({ kind }, path);
        const fallback = unstaged.files[0] ? ("unstaged" as const) : ("staged" as const);
        const first = firstInteresting(lists[fallback]);
        if (first) return await get().open({ kind: fallback }, first);
      }
      set({ target: null, path: null, file: null, raw: null, blame: null, activeBlock: -1 });
    } catch (error) {
      report(error);
    }
  },

  open: async (target, path) => {
    const token = ++generation;
    const samePlace = sameTarget(get().target, target) && get().path === path;
    set(samePlace ? { target, path } : { target, path, raw: null, blame: null });
    try {
      const [file, view] = await Promise.all([ipc.fileDiff(target, path), loadView(get().mode, target, path)]);
      if (token !== generation) return;
      const blocks = visibleBlocks(file, get().showMoved);
      const active = samePlace && blocks.some((block) => block.id === get().activeBlock) ? get().activeBlock : (blocks[0]?.id ?? -1);
      const activeHunk = samePlace ? clamp(get().activeHunk, view.raw?.hunks.length ?? 1) : 0;
      set({ file, ...view, activeBlock: active, activeHunk, expanded: samePlace ? get().expanded : new Set() });
    } catch (error) {
      report(error);
    }
  },

  setMode: (mode) => {
    set({ mode });
    const { target, path } = get();
    if (mode === "structural" || !target || !path) return;
    loadView(mode, target, path)
      .then((view) => get().path === path && get().mode === mode && set({ ...view, activeHunk: 0 }))
      .catch(report);
  },

  toggleMoved: () => {
    const showMoved = !get().showMoved;
    const blocks = visibleBlocks(get().file, showMoved);
    const active = blocks.some((block) => block.id === get().activeBlock) ? get().activeBlock : (blocks[0]?.id ?? -1);
    set({ showMoved, activeBlock: active });
  },

  toggleExpanded: (block) => {
    const expanded = new Set(get().expanded);
    if (!expanded.delete(block)) expanded.add(block);
    set({ expanded });
  },

  setActiveBlock: (block) => set({ activeBlock: block }),
  setActiveHunk: (hunk) => set({ activeHunk: hunk }),

  /** J and K: the next declaration in the structural view, the next hunk in the raw one. */
  stepBlock: (delta) => {
    const { mode, raw, file, showMoved, activeBlock, activeHunk } = get();
    if (mode === "raw") return set({ activeHunk: clamp(activeHunk + delta, raw?.hunks.length ?? 1) });
    const blocks = visibleBlocks(file, showMoved);
    const next = blocks[clamp(blocks.findIndex((block) => block.id === activeBlock) + delta, blocks.length)];
    if (next) set({ activeBlock: next.id });
  },
}));

useRepoStore.subscribe((state, previous) => {
  if (state.selection !== previous.selection) void useDiffStore.getState().sync();
});
