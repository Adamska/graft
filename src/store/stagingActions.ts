import type { LineSelection } from "../lib/actionTypes";
import { errorMessage, ipc } from "../lib/ipc";
import { selectionSize } from "../lib/rawDiff";
import type { Block } from "../lib/types";
import { useDiffStore } from "./useDiffStore";
import { useRepoStore } from "./useRepoStore";
import { useUiStore } from "./useUiStore";

/** Runs an index or working tree mutation, then refreshes the status and the open diff. */
async function mutate(action: () => Promise<unknown>): Promise<boolean> {
  try {
    await action();
    return true;
  } catch (error) {
    useUiStore.getState().notify("error", errorMessage(error));
    return false;
  } finally {
    await useRepoStore.getState().refreshStatus();
    await useDiffStore.getState().sync();
  }
}

const plural = (count: number, noun: string) => `${count} ${noun}${count === 1 ? "" : "s"}`;

export function setStaged(paths: string[], staged: boolean): Promise<boolean> {
  return mutate(() => (staged ? ipc.stagePaths(paths) : ipc.unstagePaths(paths)));
}

/** Stages or unstages one declaration of the open file, depending on which list it is shown from. */
export async function toggleBlockStage(block: Block): Promise<void> {
  const { target, path } = useDiffStore.getState();
  if (!path || !block.stageable || !target || target.kind === "commit") return;
  const selector = {
    qualified: block.qualified,
    old_start: block.old && block.old.path === null ? block.old.start : null,
    new_start: block.new && block.new.path === null ? block.new.start : null,
  };
  await mutate(() => (target.kind === "unstaged" ? ipc.stageEntity(path, selector) : ipc.unstageEntity(path, selector)));
}

/** Stages or unstages the given raw diff lines of the open file: a hunk, or a hand-picked set. */
export async function toggleLinesStage(selection: LineSelection): Promise<void> {
  const { target, path } = useDiffStore.getState();
  if (!path || !target || target.kind === "commit" || selectionSize(selection) === 0) return;
  await mutate(() => (target.kind === "unstaged" ? ipc.stageLines(path, selection) : ipc.unstageLines(path, selection)));
}

/** Asks first: discarded lines exist nowhere else. */
export function discardLines(selection: LineSelection): void {
  const { target, path } = useDiffStore.getState();
  if (!path || target?.kind !== "unstaged" || selectionSize(selection) === 0) return;
  useUiStore.getState().ask({
    title: `Discard ${plural(selectionSize(selection), "line")}?`,
    message: `The selected changes in ${path} will be removed from disk. This cannot be undone.`,
    confirm: "Discard",
    danger: true,
    onConfirm: () => mutate(() => ipc.discardLines(path, selection)),
  });
}

export function discardPaths(paths: string[]): void {
  if (paths.length === 0) return;
  useUiStore.getState().ask({
    title: paths.length === 1 ? `Discard changes to ${paths[0]}?` : `Discard changes to ${paths.length} files?`,
    message: "Tracked files return to their staged version and untracked files are deleted. This cannot be undone.",
    confirm: "Discard",
    danger: true,
    onConfirm: () => mutate(() => ipc.discardPaths(paths)),
  });
}

/** Commits the index, or rewrites the last commit when amending. */
export async function commit(message: string, amend: boolean): Promise<boolean> {
  try {
    const oid = await ipc.commitStaged(message, amend);
    useUiStore.getState().notify("info", `${amend ? "Amended" : "Committed"} ${oid.slice(0, 7)}`);
    await useRepoStore.getState().reload();
    await useDiffStore.getState().sync();
    return true;
  } catch (error) {
    useUiStore.getState().notify("error", errorMessage(error));
    return false;
  }
}
