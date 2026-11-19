import type { ReactNode } from "react";
import { actions } from "../../lib/actions";
import { refActions } from "../../lib/refActions";
import { useRepoStore } from "../../store/useRepoStore";
import { type MenuItem, useUiStore } from "../../store/useUiStore";
import { BranchIcon, ChevronIcon, FetchIcon, PullIcon, PushIcon, SpinnerIcon, StashIcon } from "../ui/icons";

/** Remote and shelf controls for the checked-out branch, with its ahead and behind counts. */
export function SyncBar() {
  const branch = useRepoStore((state) => state.repo?.head.branch ?? null);
  const tracking = useRepoStore((state) => state.tracking.find((entry) => entry.branch === branch));
  const stash = useRepoStore((state) => state.repo?.stashes[0]);
  const dirty = useRepoStore((state) => state.status.staged.length + state.status.unstaged.length > 0);
  const busy = useUiStore((state) => state.busy);
  const openMenu = useUiStore((state) => state.openMenu);

  const below = (event: React.MouseEvent<HTMLElement>, items: MenuItem[]) => {
    const bounds = (event.currentTarget.closest("[data-tool]") ?? event.currentTarget).getBoundingClientRect();
    openMenu({ clientX: bounds.left, clientY: bounds.bottom + 4, preventDefault: () => event.preventDefault() }, items);
  };
  const pullMenu: MenuItem[] = [
    { label: "Pull (fast-forward only)", run: () => actions.pull("fast_forward") },
    { label: "Pull with Rebase", run: () => actions.pull("rebase") },
    { label: "Pull with Merge", run: () => actions.pull("merge") },
  ];
  const pushMenu: MenuItem[] = [
    { label: tracking ? "Push" : "Publish Branch", run: () => actions.push() },
    { label: "Force Push…", danger: true, disabled: !tracking, run: () => actions.forcePush() },
  ];

  return (
    <div className="flex shrink-0 items-center gap-0.5" aria-busy={busy !== null}>
      <Tool label="Fetch" title="Fetch all remotes and prune deleted branches" disabled={busy !== null} onClick={() => void actions.fetch()}>
        {busy === "Fetching" ? <SpinnerIcon /> : <FetchIcon />}
      </Tool>
      <Tool
        label="Pull"
        title={tracking ? `Pull ${tracking.upstream} into ${branch}` : "The current branch tracks no remote branch"}
        count={tracking?.behind}
        disabled={busy !== null || !tracking || tracking.gone}
        onClick={() => void actions.pull()}
        onMenu={(event) => below(event, pullMenu)}
      >
        {busy === "Pulling" ? <SpinnerIcon /> : <PullIcon />}
      </Tool>
      <Tool
        label={tracking || !branch ? "Push" : "Publish"}
        title={tracking ? `Push ${branch} to ${tracking.upstream}` : "Publish this branch to the default remote"}
        count={tracking?.ahead}
        disabled={busy !== null || !branch}
        onClick={() => void actions.push()}
        onMenu={(event) => below(event, pushMenu)}
      >
        {busy === "Pushing" ? <SpinnerIcon /> : <PushIcon />}
      </Tool>
      <span className="mx-1.5 h-4 w-px bg-line-strong" />
      <Tool label="Branch" title="Create a branch at the current commit" disabled={busy !== null} onClick={() => refActions.createBranch()}>
        <BranchIcon />
      </Tool>
      <Tool label="Stash" title="Shelve all uncommitted changes" disabled={busy !== null || !dirty} onClick={() => refActions.stash()}>
        <StashIcon />
      </Tool>
      <Tool label="Pop" title={stash ? `Restore "${stash.message}" and remove it from the stash list` : "There is no stash to restore"} disabled={busy !== null || !stash} onClick={() => stash && void refActions.applyStash(stash, true)}>
        <PushIcon style={{ transform: "rotate(180deg)" }} />
      </Tool>
      {busy && !["Fetching", "Pulling", "Pushing"].includes(busy) && (
        <span className="ml-2 flex items-center gap-1.5 text-[11.5px] text-slate">
          <SpinnerIcon width={12} height={12} />
          {busy}
        </span>
      )}
    </div>
  );
}

interface ToolProps {
  label: string;
  title: string;
  count?: number;
  disabled: boolean;
  onClick: () => void;
  onMenu?: (event: React.MouseEvent<HTMLElement>) => void;
  children: ReactNode;
}

function Tool({ label, title, count, disabled, onClick, onMenu, children }: ToolProps) {
  return (
    <span data-tool className="flex items-center rounded-control text-soft hover:bg-hover has-[:disabled]:hover:bg-transparent">
      <button type="button" title={title} disabled={disabled} onClick={onClick} onContextMenu={onMenu} className="flex h-7 items-center gap-1.5 pl-2 pr-1.5 text-[12px] hover:text-ink disabled:text-faint">
        {children}
        <span className="max-[1240px]:hidden">{label}</span>
        {count !== undefined && count > 0 && <span className="rounded-full bg-accent/20 px-1.5 text-[10.5px] font-medium tabular-nums leading-[16px] text-accent">{count}</span>}
      </button>
      {onMenu && (
        <button type="button" aria-label={`${label} options`} disabled={disabled} onClick={onMenu} className="flex h-7 items-center pr-1.5 text-slate hover:text-ink disabled:text-faint">
          <ChevronIcon open width={9} height={9} />
        </button>
      )}
    </span>
  );
}
