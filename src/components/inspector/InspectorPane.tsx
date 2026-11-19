import { useDiffStore } from "../../store/useDiffStore";
import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { BlameView } from "../diff/BlameView";
import { RawDiffView } from "../diff/RawDiffView";
import { StructuralDiffView } from "../diff/StructuralDiffView";
import { COMMIT_BAR_INSET, CommitBar } from "./CommitBar";
import { CommitHeader } from "./CommitHeader";
import { DiffToolbar } from "./DiffToolbar";
import { FilePanel } from "./FilePanel";
import { OperationBanner } from "./OperationBanner";

/** Right pane: what changed in the selected commit or working tree, and how. */
export function InspectorPane() {
  const selection = useRepoStore((state) => state.selection);
  const details = useRepoStore((state) => state.details);
  const operation = useRepoStore((state) => state.repo?.operation ?? null);
  const setFocus = useUiStore((state) => state.setFocus);
  const focused = useUiStore((state) => state.focus === "inspector");
  const working = selection.kind === "working";

  return (
    <section className="relative flex min-w-0 flex-1 flex-col bg-canvas" onPointerDown={() => setFocus("inspector")} aria-label="Inspector">
      {working && operation && <OperationBanner operation={operation} />}
      {selection.kind === "commit" && details?.oid === selection.oid && <CommitHeader details={details} />}
      <FilePanel />
      <DiffToolbar />
      <div className="min-h-0 flex-1">
        <DiffBody bottomInset={working ? COMMIT_BAR_INSET : 0} />
      </div>
      {working && <CommitBar />}
      {focused && <span className="pointer-events-none absolute inset-x-0 top-0 z-30 h-px bg-accent/70" />}
    </section>
  );
}

function DiffBody({ bottomInset }: { bottomInset: number }) {
  const file = useDiffStore((state) => state.file);
  const raw = useDiffStore((state) => state.raw);
  const blame = useDiffStore((state) => state.blame);
  const mode = useDiffStore((state) => state.mode);
  const kind = useDiffStore((state) => state.target?.kind ?? "commit");
  const showMoved = useDiffStore((state) => state.showMoved);

  if (!file) return <Empty text="Nothing to compare. Select a commit, or change a file in the working tree." />;
  if (file.binary) return <Empty text="Binary or oversized file. Graft does not diff its contents." />;
  if (mode === "raw") return raw ? <RawDiffView raw={raw} kind={kind} bottomInset={bottomInset} /> : null;
  if (mode === "blame") return blame ? <BlameView blame={blame} bottomInset={bottomInset} /> : <Empty text="Tracing the history of every line…" />;
  if (file.blocks.length === 0) return <Empty text="No structural differences. Only whitespace or comments changed; switch to Raw to see them." />;
  if (!showMoved && file.blocks.every((block) => block.kind === "moved" || block.kind === "origin")) {
    return <Empty text="Every change in this file is a pure move. Turn moved blocks back on to see them." />;
  }
  return <StructuralDiffView file={file} bottomInset={bottomInset} />;
}

function Empty({ text }: { text: string }) {
  return <p className="max-w-[440px] px-4 py-5 text-[12.5px] leading-relaxed text-slate">{text}</p>;
}
