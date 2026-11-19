import { errorMessage, ipc } from "../../lib/ipc";
import { graphStore, useRepoStore } from "../../store/useRepoStore";
import { useAssistStore } from "../../store/useAssistStore";
import { useDiffStore } from "../../store/useDiffStore";
import { useUiStore } from "../../store/useUiStore";

export type PaletteGroup = "Action" | "Branch" | "Tag" | "File" | "Commit";

export interface PaletteItem {
  id: string;
  group: PaletteGroup;
  label: string;
  hint?: string;
  /** Changes the repository; ranked below navigation so a bare search never lands on it first. */
  mutating?: boolean;
  run: () => void | Promise<void>;
}

/** Runs a mutating Git command, then reloads everything derived from the repository. */
async function mutate(action: () => Promise<void>, done: string): Promise<void> {
  const { notify } = useUiStore.getState();
  try {
    await action();
    await useRepoStore.getState().reload();
    await useDiffStore.getState().sync();
    notify("info", done);
  } catch (error) {
    notify("error", errorMessage(error));
  }
}

/** Everything the palette can do right now, derived from the current stores. */
export function buildItems(): PaletteItem[] {
  const repo = useRepoStore.getState();
  const diff = useDiffStore.getState();
  const assist = useAssistStore.getState();
  const ui = useUiStore.getState();
  const items: PaletteItem[] = [
    { id: "open", group: "Action", label: "Open Repository", hint: "⌘O", run: repo.pickRepository },
    { id: "demo", group: "Action", label: "Open Built-In Refactor Demo Repo", run: repo.openDemo },
    { id: "assistant", group: "Action", label: "Configure AI Provider", run: () => ui.setSettingsOpen(true) },
  ];
  if (!repo.repo) return items;

  const working = repo.selection.kind === "working";
  const paths = (files: { path: string }[]) => files.map((file) => file.path);
  items.push(
    { id: "moved", group: "Action", label: "Toggle Moved Blocks", hint: diff.showMoved ? "shown" : "hidden", run: diff.toggleMoved },
    { id: "mode", group: "Action", label: diff.mode === "raw" ? "Show Structural AST Diff" : "Show Raw Unified Diff", hint: "Tab", run: () => diff.setMode(diff.mode === "raw" ? "structural" : "raw") },
    { id: "review", group: "Action", label: "Run AI Code Review", hint: "⌘⇧R", run: assist.review },
    { id: "generate", group: "Action", label: "Generate AI Commit Message", run: async () => {
        repo.selectWorking();
        await assist.generate();
      } },
    { id: "working", group: "Action", label: "Go to Working Tree", run: repo.selectWorking },
    { id: "refresh", group: "Action", label: "Refresh Repository", run: () => mutate(async () => undefined, "Repository refreshed") },
    { id: "sidebar", group: "Action", label: "Toggle Sidebar", hint: "⌘B", run: ui.toggleSidebar },
  );
  if (working && repo.status.unstaged.length > 0) {
    items.push({ id: "stage-all", group: "Action", label: "Stage All Changes", run: () => diff.setStaged(paths(repo.status.unstaged), true) });
  }
  if (working && repo.status.staged.length > 0) {
    items.push({ id: "unstage-all", group: "Action", label: "Unstage All Changes", run: () => diff.setStaged(paths(repo.status.staged), false) });
  }
  if (repo.selection.kind === "commit") {
    const { oid } = repo.selection;
    const short = oid.slice(0, 7);
    items.push(
      { id: "pick", group: "Action", mutating: true, label: `Cherry-pick ${short} onto ${repo.repo.head.branch ?? "HEAD"}`, run: () => mutate(() => ipc.cherryPick(oid), `Cherry-picked ${short}`) },
      { id: "detach", group: "Action", mutating: true, label: `Checkout commit ${short} (detached)`, run: () => mutate(() => ipc.checkout(oid, true), `Checked out ${short}`) },
    );
  }

  repo.repo.refs.forEach((ref, index) => {
    const group = ref.kind === "tag" ? "Tag" : "Branch";
    items.push({ id: `go:${ref.kind}:${ref.name}`, group, label: ref.name, hint: ref.is_head ? "HEAD" : ref.oid.slice(0, 7), run: () => repo.selectRef(index) });
    if (ref.kind === "local" && !ref.is_head) {
      items.push({ id: `checkout:${ref.name}`, group: "Action", mutating: true, label: `Checkout branch ${ref.name}`, run: () => mutate(() => ipc.checkout(ref.name, false), `Switched to ${ref.name}`) });
    }
  });

  for (const kind of ["commit", "unstaged", "staged"] as const) {
    for (const file of diff.summaries[kind]?.files ?? []) {
      const target = kind === "commit" ? diff.target : { kind };
      if (target) items.push({ id: `file:${kind}:${file.path}`, group: "File", label: file.path, hint: kind === "commit" ? file.status : kind, run: () => diff.open(target, file.path) });
    }
  }
  return items;
}

/** Commit matches from the Rust-side index, turned into navigable items. */
export async function searchCommitItems(query: string, limit: number): Promise<PaletteItem[]> {
  if (query.trim().length < 2 || !useRepoStore.getState().repo) return [];
  const rows = await ipc.searchCommits(query, limit).catch(() => new Uint32Array());
  const model = graphStore.model;
  return Array.from(rows, (row) => ({
    id: `commit:${row}`,
    group: "Commit" as const,
    label: model.textOf(row).summary,
    hint: model.oid(row).slice(0, 7),
    run: () => useRepoStore.getState().selectCommit(row),
  }));
}
