import { actions, currentBranch } from "../../lib/actions";
import { shortOid } from "../../lib/format";
import { refActions } from "../../lib/refActions";
import { useRepoStore } from "../../store/useRepoStore";
import type { MenuItem } from "../../store/useUiStore";

/** Everything that can be done with one commit, relative to the checked-out branch. */
export function commitMenu(oid: string): MenuItem[] {
  const repo = useRepoStore.getState().repo;
  if (!repo) return [];
  const short = shortOid(oid);
  const head = currentBranch();
  const isHead = repo.head.oid === oid;
  const branches = repo.refs.filter((ref) => ref.kind === "local" && ref.oid === oid && !ref.is_head);

  return [
    ...branches.map((ref) => ({ label: `Check Out ${ref.name}`, run: () => actions.checkout(ref.name) })),
    { label: `Check Out ${short} (detached)`, disabled: isHead && repo.head.branch === null, run: () => actions.checkoutCommit(oid) },
    { label: "New Branch Here…", run: () => refActions.createBranch(oid, short) },
    { label: "New Tag Here…", run: () => refActions.createTag(oid, short) },

    { label: `Cherry-Pick onto ${head}`, separated: true, disabled: isHead, run: () => actions.cherryPick(oid) },
    { label: "Revert Commit", run: () => actions.revert(oid) },
    { label: `Merge into ${head}`, disabled: isHead, run: () => actions.merge(oid) },
    { label: `Rebase ${head} onto This Commit`, disabled: isHead, run: () => actions.rebase(oid) },

    { label: `Reset ${head} to Here, Keep Changes Staged`, separated: true, disabled: isHead, run: () => actions.reset(oid, "soft") },
    { label: `Reset ${head} to Here, Keep Changes`, disabled: isHead, run: () => actions.reset(oid, "mixed") },
    { label: `Reset ${head} to Here, Discard Changes…`, danger: true, run: () => actions.reset(oid, "hard") },

    { label: "Copy Commit Hash", separated: true, hint: short, run: () => actions.copy(oid, "commit hash") },
  ];
}
