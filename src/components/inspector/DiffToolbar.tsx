import { directory, fileName } from "../../lib/format";
import { findingsFor, targetKey, useAssistStore } from "../../store/useAssistStore";
import { type DiffMode, useDiffStore } from "../../store/useDiffStore";
import { useRepoStore } from "../../store/useRepoStore";
import { HistoryIcon } from "../ui/icons";

const MODES: { mode: DiffMode; label: string; title: string }[] = [
  { mode: "structural", label: "Structural", title: "Syntax tree diff with moved code tracked (Tab)" },
  { mode: "raw", label: "Raw", title: "Line diff with hunk and line staging (Tab)" },
  { mode: "blame", label: "Blame", title: "Who last changed each line (B)" },
];

export function DiffToolbar() {
  const file = useDiffStore((state) => state.file);
  const mode = useDiffStore((state) => state.mode);
  const showMoved = useDiffStore((state) => state.showMoved);
  const setMode = useDiffStore((state) => state.setMode);
  const toggleMoved = useDiffStore((state) => state.toggleMoved);
  const reviewing = useAssistStore((state) => state.reviewing);
  const review = useAssistStore((state) => state.review);
  const target = useDiffStore((state) => state.target);
  const reviewed = useAssistStore((state) => state.reviewedTarget !== null && state.reviewedTarget === targetKey(target));
  const risks = useAssistStore((state) => findingsFor(state, target, file?.path ?? null).length);
  const tracingHistory = useRepoStore((state) => file !== null && state.historyPath === file.path);
  const showHistory = useRepoStore((state) => state.showHistory);
  if (!file) return null;

  const moved = file.blocks.filter((block) => block.kind === "moved").length;
  const detail = mode === "blame" ? "last change per line" : file.structural && mode === "structural" ? `${file.language} syntax tree` : "line diff";
  return (
    <div className="flex h-9 shrink-0 items-center gap-3 border-b border-line bg-surface px-3.5 text-[12px]">
      <span className="min-w-0 truncate font-mono">
        <span className="text-faint">{directory(file.path)}</span>
        <span className="text-ink">{fileName(file.path)}</span>
      </span>
      <span className="shrink-0 text-[11px] text-faint max-[1100px]:hidden">{detail}</span>

      <span className="ml-auto flex shrink-0 items-center gap-3">
        <button
          type="button"
          onClick={() => void showHistory(tracingHistory ? null : file.path)}
          aria-pressed={tracingHistory}
          title="Show only the commits that touched this file"
          className={`flex items-center gap-1.5 text-[11.5px] ${tracingHistory ? "text-accent" : "text-slate hover:text-ink"}`}
        >
          <HistoryIcon width={12} height={12} />
          History
        </button>
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
            <span className={`size-2 rounded-[2px] border ${showMoved ? "border-moved bg-moved" : "border-slate"}`} />
            {moved} moved {moved === 1 ? "block" : "blocks"}
          </button>
        )}
        <span className="flex overflow-hidden rounded-control border border-line-strong" role="tablist" aria-label="Diff mode">
          {MODES.map((entry) => (
            <button
              key={entry.mode}
              type="button"
              role="tab"
              title={entry.title}
              aria-selected={mode === entry.mode}
              onClick={() => setMode(entry.mode)}
              className={`px-2.5 text-[11.5px] leading-[20px] ${mode === entry.mode ? "bg-active text-ink" : "text-slate hover:text-ink"}`}
            >
              {entry.label}
            </button>
          ))}
        </span>
      </span>
    </div>
  );
}
