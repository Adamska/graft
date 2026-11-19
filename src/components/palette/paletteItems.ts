import { actions, currentBranch, OPERATION_LABELS } from "../../lib/actions";
import { ipc } from "../../lib/ipc";
import { refActions } from "../../lib/refActions";
import { discardPaths, setStaged } from "../../store/stagingActions";
import { useAssistStore } from "../../store/useAssistStore";
import { useDiffStore } from "../../store/useDiffStore";
import { graphStore, useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";

export type PaletteGroup = "Action" | "Branch" | "Tag" | "Stash" | "File" | "Commit";

export interface PaletteItem {
  id: string;
  group: PaletteGroup;
  label: string;
  hint?: string;
  /** Changes the repository; ranked below navigation so a bare search never lands on it first. */
  mutating?: boolean;
  run: () => unknown;
}

const action = (id: string, label: string, run: () => unknown, hint?: string): PaletteItem => ({ id, group: "Action", label, hint, run });
const change = (id: string, label: string, run: () => unknown, hint?: string): PaletteItem => ({ ...action(id, label, run, hint), mutating: true });

/** Commands that act on the repository as a whole or on the checked-out branch. */
function repositoryItems(): PaletteItem[] {
  const repo = useRepoStore.getState();
  const diff = useDiffStore.getState();
  const assist = useAssistStore.getState();
  const ui = useUiStore.getState();
  const stash = repo.repo?.stashes[0];
  const operation = repo.repo?.operation;
  const paths = (files: { path: string }[]) => files.map((file) => file.path);
  const items = [
    change("fetch", "Fetch All Remotes", actions.fetch),
    change("pull", "Pull", () => actions.pull(), "fast-forward"),
    change("pull-rebase", "Pull with Rebase", () => actions.pull("rebase")),
    change("pull-merge", "Pull with Merge", () => actions.pull("merge")),
    change("push", "Push", () => actions.push()),
    change("force-push", "Force Push", () => actions.forcePush(), "with lease"),
    change("branch", "Create Branch", () => refActions.createBranch()),
    change("stash", "Stash Changes", refActions.stash),
    change("undo", "Undo Last Commit", actions.undoCommit, "keeps changes staged"),
    action("moved", "Toggle Moved Blocks", diff.toggleMoved, diff.showMoved ? "shown" : "hidden"),
    action("structural", "Show Structural AST Diff", () => diff.setMode("structural"), "Tab"),
    action("raw", "Show Raw Unified Diff", () => diff.setMode("raw"), "Tab"),
    action("blame", "Show Blame", () => diff.setMode("blame"), "B"),
    action("review", "Run AI Code Review", assist.review, "⌘⇧R"),
    action("generate", "Generate AI Commit Message", async () => {
      repo.selectWorking();
      await assist.generate();
    }),
    action("working", "Go to Working Tree", repo.selectWorking),
    action("refresh", "Refresh Repository", async () => {
      await repo.reload();
      await diff.sync();
    }),
    action("sidebar", "Toggle Sidebar", ui.toggleSidebar, "⌘B"),
  ];
  if (stash) {
    items.push(change("pop", "Pop Latest Stash", () => refActions.applyStash(stash, true)), change("apply", "Apply Latest Stash", () => refActions.applyStash(stash, false)));
  }
  if (operation) {
    const label = OPERATION_LABELS[operation];
    items.push(change("continue", `Continue ${label}`, () => actions.advance("continue")), change("abort", `Abort ${label}`, () => actions.advance("abort")));
  }
  if (diff.path && diff.file) {
    const path = diff.path;
    items.push(action("history", `Show History of ${path}`, () => repo.showHistory(path)));
  }
  if (repo.historyPath) items.push(action("history-off", "Show All Commits", () => repo.showHistory(null)));
  if (repo.status.unstaged.length > 0) {
    items.push(change("stage-all", "Stage All Changes", () => setStaged(paths(repo.status.unstaged), true)), change("discard-all", "Discard All Changes", () => discardPaths(paths(repo.status.unstaged))));
  }
  if (repo.status.staged.length > 0) items.push(change("unstage-all", "Unstage All Changes", () => setStaged(paths(repo.status.staged), false)));
  return items;
}

function commitItems(oid: string): PaletteItem[] {
  const short = oid.slice(0, 7);
  const head = currentBranch();
  return [
    change("pick", `Cherry-pick ${short} onto ${head}`, () => actions.cherryPick(oid)),
    change("revert", `Revert ${short}`, () => actions.revert(oid)),
    change("detach", `Checkout commit ${short} (detached)`, () => actions.checkoutCommit(oid)),
    change("branch-here", `Create Branch at ${short}`, () => refActions.createBranch(oid, short)),
    change("tag-here", `Create Tag at ${short}`, () => refActions.createTag(oid, short)),
    change("reset-soft", `Reset ${head} to ${short}, keep changes staged`, () => actions.reset(oid, "soft")),
    change("reset-mixed", `Reset ${head} to ${short}, keep changes`, () => actions.reset(oid, "mixed")),
    change("reset-hard", `Reset ${head} to ${short}, discard changes`, () => actions.reset(oid, "hard")),
    action("copy-hash", `Copy Hash ${short}`, () => actions.copy(oid, "commit hash")),
  ];
}

/** Everything the palette can do right now, derived from the current stores. */
export function buildItems(): PaletteItem[] {
  const repo = useRepoStore.getState();
  const diff = useDiffStore.getState();
  const items: PaletteItem[] = [
    action("open", "Open Repository", repo.pickRepository, "⌘O"),
    action("demo", "Open Built-In Refactor Demo Repo", repo.openDemo),
    action("assistant", "Configure AI Provider", () => useUiStore.getState().setSettingsOpen(true)),
  ];
  if (!repo.repo) return items;
  items.push(...repositoryItems());
  if (repo.selection.kind === "commit") items.push(...commitItems(repo.selection.oid));

  const head = currentBranch();
  repo.repo.refs.forEach((ref, index) => {
    const group = ref.kind === "tag" ? "Tag" : "Branch";
    items.push({ id: `go:${ref.kind}:${ref.name}`, group, label: ref.name, hint: ref.is_head ? "HEAD" : ref.oid.slice(0, 7), run: () => repo.selectRef(index) });
    if (ref.is_head || ref.kind === "tag") return;
    if (ref.kind === "remote") return void items.push(change(`track:${ref.name}`, `Checkout remote branch ${ref.name}`, () => actions.checkoutRemote(ref.name)));
    items.push(
      change(`checkout:${ref.name}`, `Checkout branch ${ref.name}`, () => actions.checkout(ref.name)),
      change(`merge:${ref.name}`, `Merge ${ref.name} into ${head}`, () => actions.merge(ref.name)),
      change(`rebase:${ref.name}`, `Rebase ${head} onto ${ref.name}`, () => actions.rebase(ref.name)),
      change(`delete:${ref.name}`, `Delete branch ${ref.name}`, () => refActions.deleteBranch(ref.name)),
    );
  });
  for (const stash of repo.repo.stashes) {
    items.push({ id: `stash:${stash.oid}`, group: "Stash", label: stash.message, hint: `stash@{${stash.index}}`, run: () => repo.selectOid(stash.oid) });
  }

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
