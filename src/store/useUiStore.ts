import { create } from "zustand";

export type FocusRegion = "graph" | "inspector";

export interface Notice {
  tone: "error" | "info";
  text: string;
}

interface UiState {
  sidebarOpen: boolean;
  graphWidth: number;
  focus: FocusRegion;
  notice: Notice | null;
  toggleSidebar: () => void;
  setGraphWidth: (width: number) => void;
  setFocus: (focus: FocusRegion) => void;
  notify: (tone: Notice["tone"], text: string) => void;
  dismissNotice: () => void;
}

export const GRAPH_WIDTH = { initial: 380, min: 300, max: 760 } as const;
const NOTICE_MILLIS = 5000;
let noticeTimer: ReturnType<typeof setTimeout> | undefined;

export const useUiStore = create<UiState>((set) => ({
  sidebarOpen: true,
  graphWidth: GRAPH_WIDTH.initial,
  focus: "graph",
  notice: null,
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setGraphWidth: (width) => set({ graphWidth: Math.min(GRAPH_WIDTH.max, Math.max(GRAPH_WIDTH.min, width)) }),
  setFocus: (focus) => set({ focus }),
  notify: (tone, text) => {
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => set({ notice: null }), NOTICE_MILLIS);
    set({ notice: { tone, text } });
  },
  dismissNotice: () => set({ notice: null }),
}));
