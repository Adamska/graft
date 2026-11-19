import { useEffect } from "react";
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

/** Space: stage or unstage the active declaration, or the whole open file. */
function toggleStage(blockLevel: boolean): void {
  const diff = useDiffStore.getState();
  if (!diff.target || diff.target.kind === "commit" || !diff.path) return;
  const block = visibleBlocks(diff.file, diff.showMoved).find((candidate) => candidate.id === diff.activeBlock);
  if (blockLevel && diff.mode === "structural" && block?.stageable) void diff.toggleBlockStage(block);
  else void diff.setStaged([diff.path], diff.target.kind === "unstaged");
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
  if (ui.paletteOpen || ui.settingsOpen || repo.phase !== "ready") return;
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
      return act(() => diff.setMode(diff.mode === "raw" ? "structural" : "raw"));
    case "/":
      return act(() => document.querySelector<HTMLInputElement>("[data-graph-filter]")?.focus());
    case "c":
      return act(() => {
        repo.selectWorking();
        requestAnimationFrame(() => document.querySelector<HTMLTextAreaElement>("[data-commit-message]")?.focus());
      });
  }
}

/** Installs the application-wide keyboard map. */
export function useGlobalKeys(): void {
  useEffect(() => {
    /* A mouse click leaves focus on the clicked button, which would swallow Space and Enter. */
    const releaseFocus = (event: MouseEvent) => {
      if (event.detail > 0 && document.activeElement instanceof HTMLButtonElement) document.activeElement.blur();
    };
    window.addEventListener("keydown", handle);
    window.addEventListener("click", releaseFocus);
    return () => {
      window.removeEventListener("keydown", handle);
      window.removeEventListener("click", releaseFocus);
    };
  }, []);
}
