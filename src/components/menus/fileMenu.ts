import { actions } from "../../lib/actions";
import type { DiffTarget, FileChange } from "../../lib/types";
import { discardPaths, setStaged } from "../../store/stagingActions";
import { useDiffStore } from "../../store/useDiffStore";
import { useRepoStore } from "../../store/useRepoStore";
import type { MenuItem } from "../../store/useUiStore";

/** Opens `path` of `target` in the given view; used by "Blame" and by keyboard commands. */
export async function openIn(mode: "blame" | "raw", target: DiffTarget, path: string): Promise<void> {
  const diff = useDiffStore.getState();
  await diff.open(target, path);
  diff.setMode(mode);
}

export function fileMenu(file: FileChange, target: DiffTarget): MenuItem[] {
  const conflicted = file.status === "conflicted";
  const working: MenuItem[] =
    target.kind === "commit"
      ? []
      : target.kind === "staged"
        ? [{ label: "Unstage File", run: () => setStaged([file.path], false) }]
        : [
            { label: conflicted ? "Mark as Resolved" : "Stage File", run: () => setStaged([file.path], true) },
            ...(conflicted
              ? [
                  { label: "Resolve Using Ours", run: () => actions.resolve(file.path, "ours") },
                  { label: "Resolve Using Theirs", run: () => actions.resolve(file.path, "theirs") },
                ]
              : []),
            { label: file.status === "untracked" ? "Delete Untracked File…" : "Discard Changes…", danger: true, run: () => discardPaths([file.path]) },
          ];

  return [
    ...working,
    { label: "Show File History", separated: working.length > 0, run: () => useRepoStore.getState().showHistory(file.path) },
    { label: "Blame", disabled: file.status === "untracked", run: () => openIn("blame", target, file.path) },
    { label: "Copy Path", separated: true, run: () => actions.copy(file.path, "path") },
  ];
}
