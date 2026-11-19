import { useEffect } from "react";
import { setStaged, toggleBlockStage, toggleLinesStage } from "../store/stagingActions";
import { useAssistStore } from "../store/useAssistStore";
import { useDiffStore, visibleBlocks } from "../store/useDiffStore";
import { graphStore, useRepoStore } from "../store/useRepoStore";
import { useUiStore } from "../store/useUiStore";

/** Moves the commit selection; the working tree sits one step above the newest commit. */
function stepCommit(delta: number): void {
  const repo = useRepoStore.getState();
  const order = repo.filterRows;
  const count = order ? order.length : graphStore.model.total;
  const current = repo.selection.kind === "commit" ? (order ? order.indexOf(repo.selection.row) : repo.selection.row) : -1;
  const next = Math.min(count - 1, current + delta);
  if (next < 0) repo.selectWorking();
  else repo.selectCommit(order ? (order[next] ?? 0) : next);
}

/** Space: stage or unstage the active declaration or hunk, or the whole open file. */
function toggleStage(inspector: boolean): void {
  const diff = useDiffStore.getState();
  if (!diff.target || diff.target.kind === "commit" || !diff.path) return;
  const block = visibleBlocks(diff.file, diff.showMoved).find((candidate) => candidate.id === diff.activeBlock);
  const hunk = diff.raw?.hunks[diff.activeHunk];
  if (inspector && diff.mode === "structural" && block?.stageable) void toggleBlockStage(block);
  else if (inspector && diff.mode === "raw" && hunk) void toggleLinesStage(hunk.changes);
  else void setStaged([diff.path], diff.target.kind === "unstaged");
}

function handle(event: KeyboardEvent): void {
  const ui = useUiStore.getState();
  const repo = useRepoStore.getState();
  const command = event.metaKey || event.ctrlKey;
  const key = event.key.toLowerCase();
  const act = (action: () => unknown) => {
    event.preventDefault();
    action();
  };

  if (command && key === "k") return act(() => ui.setPaletteOpen(!ui.paletteOpen));
  if (command && key === "o") return act(() => void repo.pickRepository());
  if (key === "escape" && (ui.dialog || ui.menu)) return act(() => (ui.menu ? ui.closeMenu() : ui.ask(null)));
  if (ui.paletteOpen || ui.settingsOpen || ui.dialog || ui.menu || repo.phase !== "ready") return;
  if (command && key === "b") return act(ui.toggleSidebar);
  if (command && event.shiftKey && key === "r") return act(() => void useAssistStore.getState().review());

  const target = event.target as HTMLElement | null;
  const typing = target?.tagName === "INPUT" || target?.tagName === "TEXTAREA";
  if (typing) {
    if (key === "escape") target?.blur();
    return;
  }
  if (command || event.altKey) return;
  if (target?.tagName === "BUTTON" && (key === " " || key === "enter")) return;

  const inspector = ui.focus === "inspector";
  const diff = useDiffStore.getState();
  switch (key) {
    case "j":
    case "arrowdown":
      return act(() => (inspector ? diff.stepBlock(1) : stepCommit(1)));
    case "k":
    case "arrowup":
      return act(() => (inspector ? diff.stepBlock(-1) : stepCommit(-1)));
    case "l":
    case "arrowright":
      return act(() => ui.setFocus("inspector"));
    case "h":
    case "arrowleft":
      return act(() => ui.setFocus("graph"));
    case " ":
      return act(() => toggleStage(inspector));
    case "tab":
      return act(() => diff.setMode(diff.mode === "structural" ? "raw" : "structural"));
    case "b":
      return act(() => diff.setMode(diff.mode === "blame" ? "structural" : "blame"));
    case "escape":
      return act(() => repo.historyPath !== null && void repo.showHistory(null));
    case "/":
      return act(() => document.querySelector<HTMLInputElement>("[data-graph-filter]")?.focus());
    case "c":
      return act(() => {
        repo.selectWorking();
        requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>("[data-commit-message]")?.focus());
      });
  }
}

/** Picks up commits, checkouts, and edits made outside the app while it was in the background. */
function refresh(): void {
  const repo = useRepoStore.getState();
  if (repo.phase !== "ready" || useUiStore.getState().busy) return;
  const before = repo.status;
  void repo.reload().then(() => {
    const after = useRepoStore.getState();
    const unchanged = JSON.stringify(before) === JSON.stringify(after.status);
    if (after.selection.kind === "working" && !unchanged) void useDiffStore.getState().sync();
  });
}

/** Installs the application-wide keyboard map and the refresh on window focus. */
export function useGlobalKeys(): void {
  useEffect(() => {
    /* A mouse click leaves focus on the clicked button, which would swallow Space and Enter. */
    const releaseFocus = (event: MouseEvent) => {
      const clicked = event.target instanceof Element ? event.target.closest("button") : null;
      if (event.detail > 0 && clicked !== null && clicked === document.activeElement) clicked.blur();
    };
    window.addEventListener("keydown", handle);
    window.addEventListener("click", releaseFocus);
    window.addEventListener("focus", refresh);
    return () => {
      window.removeEventListener("keydown", handle);
      window.removeEventListener("click", releaseFocus);
      window.removeEventListener("focus", refresh);
    };
  }, []);
}
