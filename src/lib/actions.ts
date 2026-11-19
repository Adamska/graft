import { useDiffStore } from "../store/useDiffStore";
import { useRepoStore } from "../store/useRepoStore";
import { useUiStore } from "../store/useUiStore";
import type { ConflictSide, MergeMode, OperationStep, PullMode, ResetMode } from "./actionTypes";
import { errorMessage, ipc } from "./ipc";
import type { Operation } from "./types";

export const OPERATION_LABELS: Record<Operation, string> = { merge: "Merge", rebase: "Rebase", cherry_pick: "Cherry-pick", revert: "Revert" };

const short = (oid: string) => oid.slice(0, 7);

/** Name of the checked-out branch, or `HEAD` when detached. */
export const currentBranch = () => useRepoStore.getState().repo?.head.branch ?? "HEAD";

/**
 * Runs a repository command, then refreshes everything derived from the repository. The
 * refresh also happens on failure: a command that stops on a conflict has still changed
 * the working tree and left an operation open.
 */
export async function perform(label: string, action: () => Promise<string | void>, done?: string): Promise<boolean> {
  const ui = useUiStore.getState();
  if (useUiStore.getState().busy) {
    ui.notify("info", "Another Git command is still running");
    return false;
  }
  ui.setBusy(label);
  let succeeded = true;
  try {
    const message = (await action()) ?? done;
    if (message) ui.notify("info", message);
  } catch (error) {
    succeeded = false;
    ui.notify("error", errorMessage(error));
  }
  await useRepoStore.getState().reload();
  await useDiffStore.getState().sync();
  ui.setBusy(null);
  return succeeded;
}

const RESET_NOTES: Record<ResetMode, string> = {
  soft: "changes kept staged",
  mixed: "changes kept in the working tree",
  hard: "all local changes discarded",
};

export const actions = {
  fetch: () => perform("Fetching", ipc.fetch),
  pull: (mode: PullMode = "fast_forward") => perform("Pulling", () => ipc.pull(mode)),
  push: (branch: string | null = null) => perform("Pushing", () => ipc.push(branch, false)),

  forcePush: (branch: string | null = null) =>
    useUiStore.getState().ask({
      title: `Force push ${branch ?? currentBranch()}?`,
      message: "Remote commits that are not part of your branch will be replaced. The push is still refused if the remote holds work you have not fetched.",
      confirm: "Force Push",
      danger: true,
      onConfirm: () => perform("Pushing", () => ipc.push(branch, true)),
    }),

  checkout: (branch: string) => perform("Checking out", () => ipc.checkout(branch, false), `Switched to ${branch}`),
  checkoutCommit: (oid: string) => perform("Checking out", () => ipc.checkout(oid, true), `Checked out ${short(oid)} (detached)`),
  checkoutRemote: (name: string) => perform("Checking out", async () => `Switched to ${await ipc.trackBranch(name)}`),

  merge: (revision: string, mode: MergeMode = "auto") =>
    perform(
      "Merging",
      () => ipc.merge(revision, mode),
      mode === "squash" ? `Squashed ${revision} into the index; commit to finish` : `Merged ${revision} into ${currentBranch()}`,
    ),
  rebase: (onto: string) => perform("Rebasing", () => ipc.rebase(onto), `Rebased ${currentBranch()} onto ${onto}`),
  cherryPick: (oid: string) => perform("Cherry-picking", () => ipc.cherryPick(oid), `Cherry-picked ${short(oid)}`),
  revert: (oid: string) => perform("Reverting", () => ipc.revert(oid), `Reverted ${short(oid)}`),

  reset: (revision: string, mode: ResetMode) => {
    const run = () => perform("Resetting", () => ipc.reset(revision, mode), `Reset ${currentBranch()} to ${short(revision)}, ${RESET_NOTES[mode]}`);
    if (mode !== "hard") return void run();
    useUiStore.getState().ask({
      title: `Hard reset ${currentBranch()} to ${short(revision)}?`,
      message: "Every uncommitted change is discarded, and commits after this one leave the branch. This cannot be undone from Graft.",
      confirm: "Reset",
      danger: true,
      onConfirm: run,
    });
  },

  /** Takes the last commit off the branch and leaves its changes staged. */
  undoCommit: () => perform("Resetting", () => ipc.reset("HEAD~1", "soft"), "Last commit undone; its changes are staged"),

  /** Continues, skips a step of, or aborts the operation in progress. */
  advance: (step: OperationStep) => {
    const operation = useRepoStore.getState().repo?.operation;
    if (!operation) return;
    const label = OPERATION_LABELS[operation];
    const done = step === "abort" ? `${label} aborted` : step === "skip" ? "Step skipped" : `${label} continued`;
    void perform(step === "abort" ? "Aborting" : "Continuing", () => ipc.advanceOperation(operation, step), done);
  },

  resolve: (path: string, side: ConflictSide) => perform("Resolving", () => ipc.resolveConflict(path, side), `Resolved ${path} with ${side}`),

  copy: (text: string, what: string) => {
    const { notify } = useUiStore.getState();
    navigator.clipboard.writeText(text).then(
      () => notify("info", `Copied ${what}`),
      () => notify("error", `Could not copy ${what}`),
    );
  },
};
