import type { ChangeKind, ChangesetSummary, DiffTarget, FileChange } from "../../lib/types";
import { useRepoStore } from "../../store/useRepoStore";
import { useDiffStore } from "../../store/useDiffStore";
import { FileList } from "./FileList";

const CHIPS: { kinds: ChangeKind[]; symbol: string; label: string; className: string }[] = [
  { kinds: ["moved", "origin"], symbol: "⇄", label: "moved", className: "text-moved" },
  { kinds: ["moved_edited"], symbol: "⇄~", label: "moved and edited", className: "text-edited" },
  { kinds: ["signature", "body", "modified"], symbol: "~", label: "modified", className: "text-soft" },
  { kinds: ["added"], symbol: "+", label: "added", className: "text-add" },
  { kinds: ["deleted"], symbol: "−", label: "deleted", className: "text-del" },
];

/** Changed files of the current selection, with per-file structural change counts. */
export function FilePanel() {
  const working = useRepoStore((state) => state.selection.kind === "working");
  const status = useRepoStore((state) => state.status);
  const summaries = useDiffStore((state) => state.summaries);
  const setStaged = useDiffStore((state) => state.setStaged);

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
      <Group title="Staged" count={status.staged.length} action="Unstage All" onAction={() => void setStaged(paths(status.staged), false)}>
        <Files kind="staged" files={status.staged} summary={summaries.staged} />
      </Group>
      <Group title="Changes" count={status.unstaged.length} action="Stage All" onAction={() => void setStaged(paths(status.unstaged), true)}>
        <Files kind="unstaged" files={status.unstaged} summary={summaries.unstaged} />
      </Group>
    </div>
  );
}

function Group(props: { title: string; count: number; action: string; onAction: () => void; children: React.ReactNode }) {
  return (
    <section className="pb-1">
      <h3 className="flex h-7 items-center gap-2 px-3.5 text-[11px] font-medium uppercase tracking-[0.06em] text-slate">
        {props.title}
        <span className="tabular-nums text-faint">{props.count}</span>
        {props.count > 0 && (
          <button type="button" onClick={props.onAction} className="ml-auto text-[11px] normal-case tracking-normal text-slate hover:text-ink">
            {props.action}
          </button>
        )}
      </h3>
      {props.count > 0 ? props.children : <p className="px-3.5 pb-1.5 text-[12px] text-faint">None</p>}
    </section>
  );
}

function Files({ kind, files, summary }: { kind: DiffTarget["kind"]; files: FileChange[]; summary: ChangesetSummary | null }) {
  const target = useDiffStore((state) => state.target);
  const path = useDiffStore((state) => state.path);
  const open = useDiffStore((state) => state.open);
  const setStaged = useDiffStore((state) => state.setStaged);
  const oid = useRepoStore((state) => (state.selection.kind === "commit" ? state.selection.oid : ""));

  return (
    <FileList
      files={files}
      selectedPath={target?.kind === kind ? path : null}
      onSelect={(file) => void open(kind === "commit" ? { kind, oid } : { kind }, file.path)}
      trailing={(file) => (
        <span className="flex shrink-0 items-center gap-2 pr-3">
          <Chips summary={summary} path={file.path} />
          {kind !== "commit" && (
            <button
              type="button"
              onClick={() => void setStaged([file.path], kind === "unstaged")}
              className="hidden border border-line-strong px-1.5 text-[11px] leading-[17px] text-ink hover:border-add/60 hover:bg-add/10 group-hover:block"
            >
              {kind === "unstaged" ? "Stage" : "Unstage"}
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
