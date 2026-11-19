import { create } from "zustand";
import { errorMessage, ipc } from "../lib/ipc";
import type { Block, ChangesetSummary, DiffRow, DiffTarget, FileDiff } from "../lib/types";
import { useRepoStore } from "./useRepoStore";
import { useUiStore } from "./useUiStore";

export type DiffMode = "structural" | "raw";
type Summaries = Record<DiffTarget["kind"], ChangesetSummary | null>;

interface DiffState {
  target: DiffTarget | null;
  path: string | null;
  summaries: Summaries;
  file: FileDiff | null;
  raw: DiffRow[] | null;
  mode: DiffMode;
  /** When off, declarations that only moved are hidden along with their origin markers. */
  showMoved: boolean;
  expanded: ReadonlySet<number>;
  activeBlock: number;
  astMicros: number | null;

  sync: () => Promise<void>;
  open: (target: DiffTarget, path: string) => Promise<void>;
  setMode: (mode: DiffMode) => void;
  toggleMoved: () => void;
  toggleExpanded: (block: number) => void;
  setActiveBlock: (block: number) => void;
  stepBlock: (delta: number) => void;
  setStaged: (paths: string[], staged: boolean) => Promise<void>;
  toggleBlockStage: (block: Block) => Promise<void>;
  commit: (message: string) => Promise<boolean>;
}

const EMPTY: Summaries = { commit: null, staged: null, unstaged: null };
let generation = 0;

const report = (error: unknown) => useUiStore.getState().notify("error", errorMessage(error));
const sameTarget = (left: DiffTarget | null, right: DiffTarget) =>
  left?.kind === right.kind && (left.kind !== "commit" || (right.kind === "commit" && left.oid === right.oid));

/** Blocks the user can step through and act on, honoring the moved-blocks toggle. */
export function visibleBlocks(file: FileDiff | null, showMoved: boolean): Block[] {
  if (!file) return [];
  return showMoved ? file.blocks : file.blocks.filter((block) => block.kind !== "moved" && block.kind !== "origin");
}

/** Opens on the first file with declaration-level changes, falling back to the first file. */
function firstInteresting(summary: ChangesetSummary): string | undefined {
  return (summary.files.find((file) => file.changes.length > 0) ?? summary.files[0])?.path;
}

export const useDiffStore = create<DiffState>((set, get) => {
  const afterMutation = async () => {
    await useRepoStore.getState().refreshStatus();
    await get().sync();
  };

  return {
    target: null,
    path: null,
    summaries: EMPTY,
    file: null,
    raw: null,
    mode: "structural",
    showMoved: true,
    expanded: new Set(),
    activeBlock: -1,
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
        set({ target: null, path: null, file: null, raw: null, activeBlock: -1 });
      } catch (error) {
        report(error);
      }
    },

    open: async (target, path) => {
      const token = ++generation;
      const samePlace = sameTarget(get().target, target) && get().path === path;
      set({ target, path, raw: null });
      try {
        const [file, raw] = await Promise.all([ipc.fileDiff(target, path), get().mode === "raw" ? ipc.rawDiff(target, path) : null]);
        if (token !== generation) return;
        const blocks = visibleBlocks(file, get().showMoved);
        const active = samePlace && blocks.some((block) => block.id === get().activeBlock) ? get().activeBlock : (blocks[0]?.id ?? -1);
        set({ file, raw, activeBlock: active, expanded: samePlace ? get().expanded : new Set() });
      } catch (error) {
        report(error);
      }
    },

    setMode: (mode) => {
      set({ mode });
      const { target, path, raw } = get();
      if (mode !== "raw" || raw || !target || !path) return;
      ipc
        .rawDiff(target, path)
        .then((rows) => get().path === path && set({ raw: rows }))
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

    stepBlock: (delta) => {
      const blocks = visibleBlocks(get().file, get().showMoved);
      const index = blocks.findIndex((block) => block.id === get().activeBlock);
      const next = blocks[Math.max(0, Math.min(blocks.length - 1, index + delta))];
      if (next) set({ activeBlock: next.id });
    },

    setStaged: async (paths, staged) => {
      try {
        await (staged ? ipc.stagePaths(paths) : ipc.unstagePaths(paths));
        await afterMutation();
      } catch (error) {
        report(error);
      }
    },

    toggleBlockStage: async (block) => {
      const { target, path } = get();
      if (!path || !block.stageable || !target || target.kind === "commit") return;
      const selector = {
        qualified: block.qualified,
        old_start: block.old && block.old.path === null ? block.old.start : null,
        new_start: block.new && block.new.path === null ? block.new.start : null,
      };
      try {
        await (target.kind === "unstaged" ? ipc.stageEntity(path, selector) : ipc.unstageEntity(path, selector));
        await afterMutation();
      } catch (error) {
        report(error);
      }
    },

    commit: async (message) => {
      try {
        const oid = await ipc.commitStaged(message);
        useUiStore.getState().notify("info", `Committed ${oid.slice(0, 7)}`);
        await useRepoStore.getState().reload();
        await get().sync();
        return true;
      } catch (error) {
        report(error);
        return false;
      }
    },
  };
});

useRepoStore.subscribe((state, previous) => {
  if (state.selection !== previous.selection) void useDiffStore.getState().sync();
});
