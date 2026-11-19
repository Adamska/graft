import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { CommitHeader } from "./CommitHeader";
import { FileList } from "./FileList";

export function InspectorPane() {
  const selection = useRepoStore((state) => state.selection);
  const details = useRepoStore((state) => state.details);
  const status = useRepoStore((state) => state.status);
  const setFocus = useUiStore((state) => state.setFocus);

  return (
    <section className="flex min-w-0 flex-1 flex-col bg-obsidian" onPointerDown={() => setFocus("inspector")} aria-label="Inspector">
      {selection.kind === "commit" ? (
        details && details.oid === selection.oid ? (
          <>
            <CommitHeader details={details} />
            <div className="min-h-0 flex-1 overflow-y-auto py-1">
              <FileList files={details.files} />
            </div>
          </>
        ) : null
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto">
          <Group title="Staged" empty="Nothing staged">
            <FileList files={status.staged} />
          </Group>
          <Group title="Changes" empty="Working tree clean">
            <FileList files={status.unstaged} />
          </Group>
        </div>
      )}
    </section>
  );
}

function Group({ title, empty, children }: { title: string; empty: string; children: React.ReactElement<{ files: unknown[] }> }) {
  const count = children.props.files.length;
  return (
    <section className="border-b border-line pb-1">
      <h3 className="flex h-8 items-center gap-2 px-3.5 text-[11px] font-medium uppercase tracking-[0.06em] text-slate">
        {title}
        <span className="tabular-nums text-faint">{count}</span>
      </h3>
      {count > 0 ? children : <p className="px-3.5 pb-2 text-[12px] text-faint">{empty}</p>}
    </section>
  );
}
