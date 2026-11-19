import { create } from "zustand";

export type FocusRegion = "graph" | "inspector";

export interface Notice {
  tone: "error" | "info";
  text: string;
}

export interface MenuItem {
  label: string;
  hint?: string;
  danger?: boolean;
  disabled?: boolean;
  /** Draws a divider above this item. */
  separated?: boolean;
  run: () => unknown;
}

export interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

export interface DialogField {
  key: string;
  label: string;
  kind: "text" | "checkbox";
  /** A required text field must be filled in before the dialog can be confirmed. */
  required?: boolean;
  initial?: string | boolean;
  placeholder?: string;
}

export type DialogValues = Record<string, string | boolean>;

/** A confirmation or a small form; `onConfirm` runs after the dialog has closed. */
export interface DialogSpec {
  title: string;
  message?: string;
  fields?: DialogField[];
  confirm: string;
  danger?: boolean;
  onConfirm: (values: DialogValues) => unknown;
}

interface UiState {
  sidebarOpen: boolean;
  graphWidth: number;
  focus: FocusRegion;
  notice: Notice | null;
  paletteOpen: boolean;
  settingsOpen: boolean;
  menu: MenuState | null;
  dialog: DialogSpec | null;
  /** Label of the repository command currently running, shown in the toolbar. */
  busy: string | null;
  setPaletteOpen: (open: boolean) => void;
  setSettingsOpen: (open: boolean) => void;
  openMenu: (event: { clientX: number; clientY: number; preventDefault: () => void }, items: MenuItem[]) => void;
  closeMenu: () => void;
  ask: (dialog: DialogSpec | null) => void;
  setBusy: (busy: string | null) => void;
  toggleSidebar: () => void;
  setGraphWidth: (width: number) => void;
  setFocus: (focus: FocusRegion) => void;
  notify: (tone: Notice["tone"], text: string) => void;
  dismissNotice: () => void;
}

export const GRAPH_WIDTH = { initial: 400, min: 300, max: 760 } as const;
const NOTICE_MILLIS = 5000;
let noticeTimer: ReturnType<typeof setTimeout> | undefined;

export const useUiStore = create<UiState>((set) => ({
  sidebarOpen: true,
  graphWidth: GRAPH_WIDTH.initial,
  focus: "graph",
  notice: null,
  paletteOpen: false,
  settingsOpen: false,
  menu: null,
  dialog: null,
  busy: null,
  setPaletteOpen: (paletteOpen) => set({ paletteOpen, menu: null }),
  setSettingsOpen: (settingsOpen) => set({ settingsOpen, paletteOpen: false }),
  openMenu: (event, items) => {
    event.preventDefault();
    set({ menu: items.length > 0 ? { x: event.clientX, y: event.clientY, items } : null });
  },
  closeMenu: () => set({ menu: null }),
  ask: (dialog) => set({ dialog, menu: null, paletteOpen: false }),
  setBusy: (busy) => set({ busy }),
  toggleSidebar: () => set((state) => ({ sidebarOpen: !state.sidebarOpen })),
  setGraphWidth: (width) => set({ graphWidth: Math.min(GRAPH_WIDTH.max, Math.max(GRAPH_WIDTH.min, width)) }),
  setFocus: (focus) => set({ focus }),
  notify: (tone, text) => {
    clearTimeout(noticeTimer);
    noticeTimer = setTimeout(() => set({ notice: null }), tone === "error" ? NOTICE_MILLIS * 2 : NOTICE_MILLIS);
    set({ notice: { tone, text } });
  },
  dismissNotice: () => set({ notice: null }),
}));
