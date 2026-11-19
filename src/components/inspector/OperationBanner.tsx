import { actions, OPERATION_LABELS } from "../../lib/actions";
import type { Operation } from "../../lib/types";
import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";

const BUTTON = "h-6 rounded-control border px-2.5 text-[11.5px] disabled:opacity-40";

/** Shown while a merge, rebase, cherry-pick, or revert is waiting for conflicts to be resolved. */
export function OperationBanner({ operation }: { operation: Operation }) {
  const conflicts = useRepoStore((state) => state.status.unstaged.filter((file) => file.status === "conflicted").length);
  const busy = useUiStore((state) => state.busy !== null);
  const label = OPERATION_LABELS[operation];

  return (
    <div role="status" className="flex shrink-0 items-center gap-2.5 border-b border-edited/25 bg-edited/[0.07] px-3.5 py-2 text-[12.5px]">
      <span className="size-1.5 shrink-0 rounded-full bg-edited" />
      <span className="min-w-0 flex-1">
        <span className="font-medium text-ink">{label} in progress.</span>{" "}
        <span className="text-soft">
          {conflicts > 0
            ? `${conflicts} conflicted ${conflicts === 1 ? "file" : "files"} left: edit the file, or right-click it to take one side.`
            : "Every conflict is resolved. Continue to finish."}
        </span>
      </span>
      <button type="button" disabled={busy} onClick={() => actions.advance("abort")} className={`${BUTTON} border-line-strong text-soft hover:border-del/60 hover:text-del`}>
        Abort
      </button>
      {operation !== "merge" && (
        <button type="button" disabled={busy} onClick={() => actions.advance("skip")} className={`${BUTTON} border-line-strong text-soft hover:text-ink`}>
          Skip
        </button>
      )}
      <button type="button" disabled={busy || conflicts > 0} onClick={() => actions.advance("continue")} className={`${BUTTON} border-transparent bg-accent font-medium text-canvas hover:bg-accent/85`}>
        Continue
      </button>
    </div>
  );
}
