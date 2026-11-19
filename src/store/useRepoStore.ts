import { open } from "@tauri-apps/plugin-dialog";
import { create } from "zustand";
import { GraphModel } from "../lib/graphModel";
import { errorMessage, ipc } from "../lib/ipc";
import type { CommitDetails, EngineMetrics, RecentRepository, RepoSnapshot, WorkingStatus } from "../lib/types";
import { useUiStore } from "./useUiStore";

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
  selection: Selection;
  details: CommitDetails | null;
  filterQuery: string;
  filterRows: Uint32Array | null;
  metrics: EngineMetrics | null;

  loadRecents: () => Promise<void>;
  pickRepository: () => Promise<void>;
  openRepository: (path: string) => Promise<void>;
  openDemo: () => Promise<void>;
  closeRepository: () => void;
  reload: () => Promise<void>;
  refreshStatus: () => Promise<void>;
  refreshMetrics: () => Promise<void>;
  selectWorking: () => void;
  selectCommit: (row: number) => void;
  selectRef: (index: number) => void;
  setFilter: (query: string) => Promise<void>;
}

const report = (error: unknown) => useUiStore.getState().notify("error", errorMessage(error));

export const useRepoStore = create<RepoState>((set, get) => {
  const streamGraph = async () => {
    const token = ++streamToken;
    const model = new GraphModel();
    graphStore.model = model;
    set((state) => ({ graphVersion: state.graphVersion + 1 }));
    await ipc.streamGraph((frame) => {
      if (token !== streamToken) return;
      model.append(frame);
      set((state) => ({ graphVersion: state.graphVersion + 1 }));
    });
  };

  return {
    phase: "welcome",
    repo: null,
    recents: [],
    graphVersion: 0,
    status: EMPTY_STATUS,
    selection: { kind: "working" },
    details: null,
    filterQuery: "",
    filterRows: null,
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
        set({ repo, phase: "ready", status: EMPTY_STATUS, selection: { kind: "working" }, details: null, filterQuery: "", filterRows: null });
        await Promise.all([streamGraph(), get().refreshStatus()]);
        const { staged, unstaged } = get().status;
        if (staged.length + unstaged.length === 0 && graphStore.model.total > 0) get().selectCommit(0);
        void get().loadRecents();
      } catch (error) {
        set({ phase: get().repo ? "ready" : "welcome" });
        report(error);
      }
    },

    openDemo: async () => {
      set({ phase: "opening" });
      try {
        await get().openRepository(await ipc.createDemoRepository());
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

    reload: async () => {
      try {
        set({ repo: await ipc.refreshRepository() });
        await Promise.all([streamGraph(), get().refreshStatus()]);
        const selection = get().selection;
        if (selection.kind === "commit" && graphStore.model.oid(selection.row) !== selection.oid) get().selectWorking();
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
      if (row < 0 || row >= model.total) return;
      const oid = model.oid(row);
      set({ selection: { kind: "commit", row, oid } });
      ipc
        .commitDetails(oid)
        .then((details) => {
          const current = get().selection;
          if (current.kind === "commit" && current.oid === oid) set({ details });
        })
        .catch(report);
    },

    selectRef: (index) => {
      const row = graphStore.model.rowByRef.get(index);
      if (row !== undefined) get().selectCommit(row);
    },

    setFilter: async (query) => {
      set({ filterQuery: query });
      if (query.trim() === "") return set({ filterRows: null });
      const rows = await ipc.searchCommits(query).catch(() => new Uint32Array());
      if (get().filterQuery === query) set({ filterRows: rows });
    },
  };
});
