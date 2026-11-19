import { actions } from "../../lib/actions";
import type { ChangeKind, ChangesetSummary, DiffTarget, FileChange } from "../../lib/types";
import { discardPaths, setStaged } from "../../store/stagingActions";
import { useDiffStore } from "../../store/useDiffStore";
import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { fileMenu } from "../menus/fileMenu";
import { FileList } from "./FileList";

const CHIPS: { kinds: ChangeKind[]; symbol: string; label: string; className: string }[] = [
  { kinds: ["moved", "origin"], symbol: "⇄", label: "moved", className: "text-moved" },
  { kinds: ["moved_edited"], symbol: "⇄~", label: "moved and edited", className: "text-edited" },
  { kinds: ["signature", "body", "modified"], symbol: "~", label: "modified", className: "text-soft" },
  { kinds: ["added"], symbol: "+", label: "added", className: "text-add" },
  { kinds: ["deleted"], symbol: "−", label: "deleted", className: "text-del" },
];

const ROW_BUTTON = "hidden rounded-control border border-line-strong px-1.5 text-[11px] leading-[17px] text-ink group-hover:block";

/** Changed files of the current selection, with per-file structural change counts. */
export function FilePanel() {
  const working = useRepoStore((state) => state.selection.kind === "working");
  const status = useRepoStore((state) => state.status);
  const summaries = useDiffStore((state) => state.summaries);

  if (!working) {
    return (
      <div className="max-h-[30%] shrink-0 overflow-y-auto border-b border-line py-1">
        <Files kind="commit" files={summaries.commit?.files ?? []} summary={summaries.commit} />
      </div>
    );
  }
  const paths = (files: FileChange[]) => files.map((file) => file.path);
  return (
    <div className="max-h-[34%] shrink-0 overflow-y-auto border-b border-line">
      <Group title="Staged" count={status.staged.length} actions={[{ label: "Unstage All", run: () => void setStaged(paths(status.staged), false) }]}>
        <Files kind="staged" files={status.staged} summary={summaries.staged} />
      </Group>
      <Group
        title="Changes"
        count={status.unstaged.length}
        actions={[
          { label: "Discard All", danger: true, run: () => discardPaths(paths(status.unstaged)) },
          { label: "Stage All", run: () => void setStaged(paths(status.unstaged), true) },
        ]}
      >
        <Files kind="unstaged" files={status.unstaged} summary={summaries.unstaged} />
      </Group>
    </div>
  );
}

interface GroupAction {
  label: string;
  danger?: boolean;
  run: () => void;
}

function Group(props: { title: string; count: number; actions: GroupAction[]; children: React.ReactNode }) {
  return (
    <section className="pb-1">
      <h3 className="flex h-7 items-center gap-3 px-3.5 text-[11px] font-medium uppercase tracking-[0.06em] text-slate">
        <span className="flex-1">
          {props.title} <span className="ml-1 tabular-nums text-faint">{props.count}</span>
        </span>
        {props.count > 0 &&
          props.actions.map((action) => (
            <button key={action.label} type="button" onClick={action.run} className={`text-[11px] normal-case tracking-normal text-slate ${action.danger ? "hover:text-del" : "hover:text-ink"}`}>
              {action.label}
            </button>
          ))}
      </h3>
      {props.count > 0 ? props.children : <p className="px-3.5 pb-1.5 text-[12px] text-faint">None</p>}
    </section>
  );
}

function Files({ kind, files, summary }: { kind: DiffTarget["kind"]; files: FileChange[]; summary: ChangesetSummary | null }) {
  const target = useDiffStore((state) => state.target);
  const path = useDiffStore((state) => state.path);
  const open = useDiffStore((state) => state.open);
  const oid = useRepoStore((state) => (state.selection.kind === "commit" ? state.selection.oid : ""));
  const openMenu = useUiStore((state) => state.openMenu);
  const own: DiffTarget = kind === "commit" ? { kind, oid } : { kind };

  return (
    <FileList
      files={files}
      selectedPath={target?.kind === kind ? path : null}
      onSelect={(file) => void open(own, file.path)}
      onMenu={(event, file) => openMenu(event, fileMenu(file, own))}
      trailing={(file) => (
        <span className="flex shrink-0 items-center gap-1.5 pr-3">
          <span className="mr-0.5">
            <Chips summary={summary} path={file.path} />
          </span>
          {kind === "unstaged" && file.status === "conflicted" && (
            <>
              <button type="button" onClick={() => void actions.resolve(file.path, "ours")} className={`${ROW_BUTTON} hover:border-accent/60 hover:bg-accent/10`}>
                Ours
              </button>
              <button type="button" onClick={() => void actions.resolve(file.path, "theirs")} className={`${ROW_BUTTON} hover:border-accent/60 hover:bg-accent/10`}>
                Theirs
              </button>
            </>
          )}
          {kind === "unstaged" && file.status !== "conflicted" && (
            <button type="button" onClick={() => discardPaths([file.path])} className={`${ROW_BUTTON} hover:border-del/60 hover:bg-del/10`}>
              Discard
            </button>
          )}
          {kind !== "commit" && (
            <button type="button" onClick={() => void setStaged([file.path], kind === "unstaged")} className={`${ROW_BUTTON} hover:border-add/60 hover:bg-add/10`}>
              {kind === "staged" ? "Unstage" : file.status === "conflicted" ? "Resolved" : "Stage"}
            </button>
          )}
        </span>
      )}
    />
  );
}

function Chips({ summary, path }: { summary: ChangesetSummary | null; path: string }) {
  const file = summary?.files.find((candidate) => candidate.path === path);
  if (!file) return null;
  if (file.binary) return <span className="font-mono text-[10.5px] text-faint">binary</span>;
  return (
    <span className="flex gap-2 font-mono text-[10.5px] tabular-nums">
      {CHIPS.map((chip) => {
        const count = file.changes.filter((change) => chip.kinds.includes(change.kind)).length;
        return count > 0 ? (
          <span key={chip.label} className={chip.className} title={`${count} ${chip.label}`}>
            {chip.symbol}
            {count}
          </span>
        ) : null;
      })}
    </span>
  );
}
