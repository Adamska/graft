import { actions } from "../../lib/actions";
import { formatTimestamp, shortOid } from "../../lib/format";
import { refActions } from "../../lib/refActions";
import type { CommitDetails } from "../../lib/types";
import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { commitMenu } from "../menus/commitMenu";
import { stashMenu } from "../menus/refMenus";
import { MoreIcon } from "../ui/icons";

const BUTTON = "h-6 shrink-0 rounded-control border border-line-strong px-2 text-[11.5px] text-ink hover:border-accent/60 hover:bg-accent/[0.06]";

export function CommitHeader({ details }: { details: CommitDetails }) {
  const stash = useRepoStore((state) => state.repo?.stashes.find((entry) => entry.oid === details.oid));
  const openMenu = useUiStore((state) => state.openMenu);
  const items = () => (stash ? stashMenu(stash) : commitMenu(details.oid));

  return (
    <header className="shrink-0 border-b border-line px-4 py-3" onContextMenu={(event) => openMenu(event, items())}>
      <div className="flex items-start gap-2">
        <h2 className="min-w-0 flex-1 select-text text-[14px] font-semibold leading-snug tracking-[-0.005em]">{details.summary}</h2>
        {stash && (
          <>
            <button type="button" onClick={() => void refActions.applyStash(stash, false)} className={BUTTON}>
              Apply
            </button>
            <button type="button" onClick={() => void refActions.applyStash(stash, true)} className={BUTTON}>
              Pop
            </button>
          </>
        )}
        <button type="button" title="Commit actions" aria-label="Commit actions" onClick={(event) => openMenu(event, items())} className="flex size-6 shrink-0 items-center justify-center rounded-control text-slate hover:bg-hover hover:text-ink">
          <MoreIcon />
        </button>
      </div>
      {details.body && <p className="mt-1.5 max-h-24 select-text overflow-y-auto whitespace-pre-wrap text-[12.5px] leading-relaxed text-slate">{details.body}</p>}
      <dl className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11.5px] text-slate">
        <dd className="select-text text-soft">{details.author}</dd>
        <dd>{formatTimestamp(details.time)}</dd>
        <dd>
          <button type="button" title="Copy the full hash" onClick={() => actions.copy(details.oid, "commit hash")} className="font-mono text-accent hover:underline">
            {shortOid(details.oid)}
          </button>
        </dd>
        {stash && <dd className="text-edited">stash</dd>}
        {details.parents.length > 0 && !stash && (
          <dd className="font-mono text-faint">
            {details.parents.length > 1 ? "merge of " : "parent "}
            {details.parents.map(shortOid).join(" + ")}
          </dd>
        )}
      </dl>
    </header>
  );
}
