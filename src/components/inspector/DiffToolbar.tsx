import { directory, fileName } from "../../lib/format";
import { findingsFor, useAssistStore } from "../../store/useAssistStore";
import { type DiffMode, useDiffStore } from "../../store/useDiffStore";

const MODES: { mode: DiffMode; label: string }[] = [
  { mode: "structural", label: "Structural" },
  { mode: "raw", label: "Raw" },
];

export function DiffToolbar() {
  const file = useDiffStore((state) => state.file);
  const mode = useDiffStore((state) => state.mode);
  const showMoved = useDiffStore((state) => state.showMoved);
  const setMode = useDiffStore((state) => state.setMode);
  const toggleMoved = useDiffStore((state) => state.toggleMoved);
  const reviewing = useAssistStore((state) => state.reviewing);
  const review = useAssistStore((state) => state.review);
  const reviewed = useAssistStore((state) => state.reviewedTarget !== null);
  const target = useDiffStore((state) => state.target);
  const risks = useAssistStore((state) => findingsFor(state, target, file?.path ?? null).length);
  if (!file) return null;

  const moved = file.blocks.filter((block) => block.kind === "moved").length;
  return (
    <div className="flex h-9 shrink-0 items-center gap-3 border-b border-line bg-surface px-3.5 text-[12px]">
      <span className="min-w-0 truncate font-mono">
        <span className="text-faint">{directory(file.path)}</span>
        <span className="text-ink">{fileName(file.path)}</span>
      </span>
      <span className="shrink-0 text-[11px] text-faint">{file.structural ? `${file.language} syntax tree` : "line diff"}</span>

      <span className="ml-auto flex shrink-0 items-center gap-3">
        <button
          type="button"
          disabled={reviewing}
          onClick={() => void review()}
          title="Audit this changeset for regressions (⌘⇧R)"
          className={`text-[11.5px] ${risks > 0 ? "text-edited" : "text-slate hover:text-ink"}`}
        >
          {reviewing ? "Reviewing" : risks > 0 ? `${risks} ${risks === 1 ? "risk" : "risks"}` : reviewed ? "Reviewed" : "Review"}
        </button>
        {file.structural && moved > 0 && mode === "structural" && (
          <button type="button" onClick={toggleMoved} aria-pressed={showMoved} className={`flex items-center gap-1.5 text-[11.5px] ${showMoved ? "text-moved" : "text-slate hover:text-ink"}`}>
            <span className={`size-2 border ${showMoved ? "border-moved bg-moved" : "border-slate"}`} />
            {moved} moved {moved === 1 ? "block" : "blocks"}
          </button>
        )}
        <span className="flex border border-line-strong" role="tablist" aria-label="Diff mode">
          {MODES.map((entry) => (
            <button
              key={entry.mode}
              type="button"
              role="tab"
              aria-selected={mode === entry.mode}
              onClick={() => setMode(entry.mode)}
              className={`px-2.5 text-[11.5px] leading-[20px] ${mode === entry.mode ? "bg-white/[0.09] text-ink" : "text-slate hover:text-ink"}`}
            >
              {entry.label}
            </button>
          ))}
        </span>
        <kbd className="font-mono text-[10.5px] text-faint">Tab</kbd>
      </span>
    </div>
  );
}
