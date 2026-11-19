import { open } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";
import type { Tracking } from "../lib/actionTypes";
import { GraphModel } from "../lib/graphModel";
import { errorMessage, ipc } from "../lib/ipc";
import type { CommitDetails, EngineMetrics, RecentRepository, RepoSnapshot, WorkingStatus } from "../lib/types";
import { useUiStore } from "./useUiStore";

/** A commit with `row < 0` is not part of the graph, which is the case for stashes. */
export type Selection = { kind: "working" } | { kind: "commit"; row: number; oid: string };

/** The graph lives outside React state: it is appended to in place while chunks stream in. */
export const graphStore = { model: new GraphModel() };
let streamToken = 0;

const EMPTY_STATUS: WorkingStatus = { staged: [], unstaged: [] };

interface RepoState {
  phase: "welcome" | "opening" | "ready";
  repo: RepoSnapshot | null;
  recents: RecentRepository[];
  /** Bumped whenever the graph model changes so views know to repaint. */
  graphVersion: number;
  status: WorkingStatus;
  tracking: Tracking[];
  selection: Selection;
  details: CommitDetails | null;
  filterQuery: string;
  filterRows: Uint32Array | null;
  /** File whose history currently narrows the graph; takes precedence over the text filter. */
  historyPath: string | null;
  metrics: EngineMetrics | null;

  loadRecents: () => Promise<void>;
  pickRepository: () => Promise<void>;
  openRepository: (path: string) => Promise<void>;
  closeRepository: () => void;
  reload: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshMetrics: () => Promise<void>;
  selectWorking: () => void;
  selectCommit: (row: number) => void;
  selectOid: (oid: string) => Promise<void>;
  selectRef: (index: number) => void;
  setFilter: (query: string) => Promise<void>;
  showHistory: (path: string | null) => Promise<void>;
}

const report = (error: unknown) => useUiStore.getState().notify("error", errorMessage(error));

/** Everything the graph layout depends on; equal keys mean the rows on screen are still valid. */
const graphKey = (repo: RepoSnapshot) => `${repo.head.oid}\n${repo.refs.map((ref) => `${ref.name} ${ref.oid}`).join("\n")}`;

export const useRepoStore = create<RepoState>((set, get) => {
  /** Streams into a fresh model and swaps it in with the first chunk, so a reload never blanks the graph. */
  const streamGraph = async () => {
    const token = ++streamToken;
    const model = new GraphModel();
    const publish = () => {
      if (token !== streamToken) return;
      graphStore.model = model;
      set((state) => ({ graphVersion: state.graphVersion + 1 }));
    };
    await ipc.streamGraph((frame) => {
      model.append(frame);
      publish();
    });
    publish();
  };

  const showCommit = (row: number, oid: string) => {
    set({ selection: { kind: "commit", row, oid } });
    ipc
      .commitDetails(oid)
      .then((details) => {
        const current = get().selection;
        if (current.kind === "commit" && current.oid === oid) set({ details });
      })
      .catch(report);
  };

  /** After a restream, row numbers are stale: reapply the filter and find the selection again. */
  const restoreView = async () => {
    const { historyPath, filterQuery, selection, repo } = get();
    if (historyPath) await get().showHistory(historyPath);
    else if (filterQuery.trim() !== "") await get().setFilter(filterQuery);
    if (selection.kind !== "commit" || graphStore.model.oid(selection.row) === selection.oid) return;
    if (selection.row < 0 && repo?.stashes.some((stash) => stash.oid === selection.oid)) return;
    const [row] = await ipc.searchCommits(selection.oid, 1).catch(() => new Uint32Array());
    if (row === undefined) get().selectWorking();
    else set({ selection: { kind: "commit", row, oid: selection.oid } });
  };

  return {
    phase: "welcome",
    repo: null,
    recents: [],
    graphVersion: 0,
    status: EMPTY_STATUS,
    tracking: [],
    selection: { kind: "working" },
    details: null,
    filterQuery: "",
    filterRows: null,
    historyPath: null,
    metrics: null,

    loadRecents: async () => set({ recents: await ipc.recentRepositories().catch(() => []) }),

    pickRepository: async () => {
      const path = await open({ directory: true, multiple: false, title: "Open Repository" });
      if (typeof path === "string") await get().openRepository(path);
    },

    openRepository: async (path) => {
      set({ phase: "opening" });
      try {
        const repo = await ipc.openRepository(path);
        graphStore.model = new GraphModel();
        set({ repo, phase: "ready", status: EMPTY_STATUS, tracking: [], selection: { kind: "working" }, details: null, filterQuery: "", filterRows: null, historyPath: null });
        void ipc.branchTracking().then((tracking) => get().repo?.path === repo.path && set({ tracking })).catch(() => undefined);
        await Promise.all([streamGraph(), get().refreshStatus()]);
        const { staged, unstaged } = get().status;
        if (staged.length + unstaged.length === 0 && graphStore.model.total > 0) get().selectCommit(0);
        void get().loadRecents();
      } catch (error) {
        set({ phase: get().repo ? "ready" : "welcome" });
        report(error);
      }
    },

    closeRepository: () => {
      streamToken++;
      set({ phase: "welcome", repo: null, details: null });
      void get().loadRecents();
    },

    /** Re-reads refs and status; the graph is only streamed again when a ref or `HEAD` moved. */
    reload: async () => {
      const previous = get().repo;
      if (!previous) return;
      try {
        const repo = await ipc.refreshRepository();
        if (get().repo?.path !== repo.path) return;
        const moved = graphKey(previous) !== graphKey(repo);
        set({ repo });
        await Promise.all([moved ? streamGraph() : null, get().refreshStatus()]);
        if (moved) await restoreView();
        set({ tracking: await ipc.branchTracking().catch(() => get().tracking) });
      } catch (error) {
        report(error);
      }
    },

    refreshStatus: async () => {
      try {
        set({ status: await ipc.workingStatus() });
      } catch (error) {
        report(error);
      }
    },

    refreshMetrics: async () => set({ metrics: await ipc.engineMetrics().catch(() => null) }),

    selectWorking: () => set({ selection: { kind: "working" }, details: null }),

    selectCommit: (row) => {
      const model = graphStore.model;
      if (row >= 0 && row < model.total) showCommit(row, model.oid(row));
    },

    /** Selects a commit by id; one that is not in the graph, such as a stash, is shown on its own. */
    selectOid: async (oid) => {
      const [row] = await ipc.searchCommits(oid, 1).catch(() => new Uint32Array());
      showCommit(row ?? -1, oid);
    },

    selectRef: (index) => {
      const row = graphStore.model.rowByRef.get(index);
      if (row !== undefined) get().selectCommit(row);
    },

    setFilter: async (query) => {
      set({ filterQuery: query, historyPath: null });
      if (query.trim() === "") return set({ filterRows: null });
      const rows = await ipc.searchCommits(query).catch(() => new Uint32Array());
      if (get().filterQuery === query && get().historyPath === null) set({ filterRows: rows });
    },

    showHistory: async (path) => {
      set({ historyPath: path, filterQuery: "" });
      if (path === null) return set({ filterRows: null });
      const rows = await ipc.fileHistory(path).catch(() => new Uint32Array());
      if (get().historyPath === path) set({ filterRows: rows });
    },
  };
});
