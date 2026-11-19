import { actions, currentBranch } from "../../lib/actions";
import type { Tracking } from "../../lib/actionTypes";
import { refActions } from "../../lib/refActions";
import type { RefEntry, Stash } from "../../lib/types";
import { useRepoStore } from "../../store/useRepoStore";
import type { MenuItem } from "../../store/useUiStore";

/** Double-click and Enter: switch to the branch, track the remote branch, or detach at the tag. */
export function activateRef(ref: RefEntry): void {
  if (ref.kind === "local") void (ref.is_head ? null : actions.checkout(ref.name));
  else if (ref.kind === "remote") void actions.checkoutRemote(ref.name);
  else void actions.checkoutCommit(ref.oid);
}

function integrate(name: string): MenuItem[] {
  const head = currentBranch();
  return [
    { label: `Merge ${name} into ${head}`, separated: true, run: () => actions.merge(name) },
    { label: `Merge ${name} with a merge commit`, run: () => actions.merge(name, "no_fast_forward") },
    { label: `Squash ${name} into ${head}`, run: () => actions.merge(name, "squash") },
    { label: `Rebase ${head} onto ${name}`, run: () => actions.rebase(name) },
  ];
}

export function refMenu(ref: RefEntry, tracking: Tracking | undefined): MenuItem[] {
  const copy: MenuItem = { label: "Copy Name", separated: true, run: () => actions.copy(ref.name, ref.name) };
  const branchHere: MenuItem = { label: "New Branch Here…", run: () => refActions.createBranch(ref.name, ref.name) };

  if (ref.kind === "tag") {
    return [
      { label: `Check Out ${ref.name} (detached)`, run: () => actions.checkoutCommit(ref.oid) },
      branchHere,
      ...integrate(ref.name),
      { label: "Push Tag", separated: true, run: () => refActions.pushTag(ref.name) },
      { label: "Delete Tag…", danger: true, run: () => refActions.deleteTag(ref.name) },
      copy,
    ];
  }
  if (ref.kind === "remote") {
    return [
      { label: `Check Out ${ref.name}`, hint: "tracking", run: () => actions.checkoutRemote(ref.name) },
      branchHere,
      ...integrate(ref.name),
      { label: "Delete from Remote…", separated: true, danger: true, run: () => refActions.deleteRemoteBranch(ref.name) },
      copy,
    ];
  }

  const sync: MenuItem[] = ref.is_head
    ? [
        { label: "Pull", separated: true, hint: tracking && tracking.behind > 0 ? `${tracking.behind} behind` : undefined, run: () => actions.pull() },
        { label: "Pull with Rebase", run: () => actions.pull("rebase") },
        { label: "Pull with Merge", run: () => actions.pull("merge") },
      ]
    : [];
  return [
    { label: `Check Out ${ref.name}`, disabled: ref.is_head, run: () => actions.checkout(ref.name) },
    branchHere,
    ...(ref.is_head ? [] : integrate(ref.name)),
    ...sync,
    { label: tracking ? "Push" : "Publish Branch", separated: !ref.is_head, hint: tracking && tracking.ahead > 0 ? `${tracking.ahead} ahead` : undefined, run: () => actions.push(ref.name) },
    { label: "Force Push…", danger: true, disabled: !tracking, run: () => actions.forcePush(ref.name) },
    { label: "Rename…", separated: true, run: () => refActions.renameBranch(ref.name) },
    { label: "Delete…", danger: true, disabled: ref.is_head, run: () => refActions.deleteBranch(ref.name) },
    copy,
  ];
}

export function stashMenu(stash: Stash): MenuItem[] {
  return [
    { label: "Show Changes", run: () => useRepoStore.getState().selectOid(stash.oid) },
    { label: "Apply", separated: true, hint: "keeps the stash", run: () => refActions.applyStash(stash, false) },
    { label: "Pop", hint: "applies and removes", run: () => refActions.applyStash(stash, true) },
    { label: "Drop…", separated: true, danger: true, run: () => refActions.dropStash(stash) },
  ];
}
