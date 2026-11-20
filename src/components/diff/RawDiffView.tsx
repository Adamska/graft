import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { LineSelection } from "../../lib/actionTypes";
import { hunkLabel, type RawDiff, type RawLine } from "../../lib/rawDiff";
import type { DiffTarget } from "../../lib/types";
import { discardLines, toggleLinesStage } from "../../store/stagingActions";
import { useDiffStore } from "../../store/useDiffStore";
import { useRepoStore } from "../../store/useRepoStore";
import { Segments } from "./Segments";
import { rowAt } from "./visualRows";

const HEIGHTS = { hunk: 30, line: 20 } as const;
const OVERSCAN = 400;
const BUTTON = "rounded-control border border-line-strong px-2 text-[11px] leading-[18px] text-ink";

const STYLES = {
  context: { sign: " ", row: "", mark: "", tone: "text-faint" },
  add: { sign: "+", row: "bg-add/[0.11]", mark: "mark-new", tone: "text-add" },
  del: { sign: "-", row: "bg-del/[0.11]", mark: "mark-old", tone: "text-del" },
} as const;

interface Props {
  raw: RawDiff;
  kind: DiffTarget["kind"];
  bottomInset: number;
}

/**
 * Classic unified diff, virtualized by row offsets. In the working tree every hunk can be
 * staged, unstaged, or discarded, and clicking line numbers picks individual lines.
 */
export function RawDiffView({ raw, kind, bottomInset }: Props) {
  const scroller = useRef<HTMLDivElement>(null);
  const anchor = useRef<number | null>(null);
  const [viewport, setViewport] = useState({ top: 0, height: 600, width: 800 });
  const [picked, setPicked] = useState<ReadonlySet<number>>(new Set());
  const activeHunk = useDiffStore((state) => state.activeHunk);
  const setActiveHunk = useDiffStore((state) => state.setActiveHunk);
  const path = useDiffStore((state) => state.path);
  const conflicted = useRepoStore((state) => state.status.unstaged.some((file) => file.status === "conflicted" && file.path === path));
  /* A conflicted file is settled as a whole: by taking a side or by editing it. */
  const editable = kind !== "commit" && !conflicted;
  const verb = kind === "staged" ? "Unstage" : "Stage";

  const offsets = useMemo(() => {
    const tops = new Float64Array(raw.lines.length + 1);
    raw.lines.forEach((line, index) => (tops[index + 1] = (tops[index] ?? 0) + (line.kind === "hunk" ? HEIGHTS.hunk : HEIGHTS.line)));
    return tops;
  }, [raw]);
  const total = offsets[raw.lines.length] ?? 0;
  const first = rowAt(offsets, viewport.top - OVERSCAN);
  const last = rowAt(offsets, viewport.top + viewport.height + OVERSCAN);

  useEffect(() => setPicked(new Set()), [raw]);

  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element) return;
    const measure = () => setViewport((current) => ({ ...current, height: element.clientHeight, width: element.clientWidth }));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  /* Keep the hunk chosen with J/K inside the viewport. */
  useEffect(() => {
    const element = scroller.current;
    const top = offsets[raw.hunks[activeHunk]?.line ?? 0] ?? 0;
    if (!element) return;
    if (top < element.scrollTop) element.scrollTo({ top: Math.max(0, top - 8) });
    else if (top > element.scrollTop + element.clientHeight - bottomInset - 80) element.scrollTo({ top: top - element.clientHeight * 0.3 });
  }, [activeHunk, raw, offsets, bottomInset]);

  const pick = (index: number, extend: boolean) => {
    const next = new Set(picked);
    const from = extend && anchor.current !== null ? anchor.current : index;
    const adding = from !== index || !next.has(index);
    for (let at = Math.min(from, index); at <= Math.max(from, index); at++) {
      const kind = raw.lines[at]?.kind;
      if (kind !== "add" && kind !== "del") continue;
      if (adding) next.add(at);
      else next.delete(at);
    }
    anchor.current = index;
    setPicked(next);
  };

  /* Double-click: the whole run of consecutive changed lines around `index`. */
  const pickRun = (index: number) => {
    const changed = (at: number) => raw.lines[at]?.kind === "add" || raw.lines[at]?.kind === "del";
    const next = new Set(picked);
    for (let at = index; changed(at); at--) next.add(at);
    for (let at = index; changed(at); at++) next.add(at);
    setPicked(next);
  };

  const selection = useMemo(() => {
    const lines: LineSelection = { old: [], new: [] };
    for (const index of picked) {
      const line = raw.lines[index];
      if (line?.kind === "del" && line.old !== null) lines.old.push(line.old);
      if (line?.kind === "add" && line.new !== null) lines.new.push(line.new);
    }
    return lines;
  }, [picked, raw]);

  const renderLine = (line: RawLine, index: number) => {
    if (line.kind === "hunk") {
      const hunk = raw.hunks[line.hunk];
      const active = editable && line.hunk === activeHunk;
      return (
        <div key={index} style={{ height: HEIGHTS.hunk }} onPointerDown={() => setActiveHunk(line.hunk)}>
          <div className={`sticky left-0 flex h-full items-center gap-2 border-y border-line pl-3 pr-3 ${active ? "bg-raised" : "bg-surface"}`} style={{ width: viewport.width }}>
            {active && <span className="absolute inset-y-0 left-0 w-0.5 bg-accent" />}
            <span className="font-mono text-[11px] text-slate">{hunk ? hunkLabel(hunk) : ""}</span>
            {editable && hunk && (
              <span className="ml-auto flex gap-1.5">
                {kind === "unstaged" && (
                  <button type="button" onClick={() => discardLines(hunk.changes)} className={`${BUTTON} hover:border-del/60 hover:bg-del/10`}>
                    Discard Hunk
                  </button>
                )}
                <button type="button" onClick={() => void toggleLinesStage(hunk.changes)} className={`${BUTTON} hover:border-add/60 hover:bg-add/10`}>
                  {verb} Hunk
                </button>
              </span>
            )}
          </div>
        </div>
      );
    }
    const style = STYLES[line.kind];
    const pickable = editable && line.kind !== "context";
    const chosen = picked.has(index);
    return (
      <div key={index} className={`code-line flex h-5 ${style.row} ${chosen ? "shadow-[inset_2px_0_0_var(--color-accent)] brightness-125" : ""}`} onPointerDown={() => setActiveHunk(line.hunk)}>
        <span
          className={`flex shrink-0 select-none text-[11px] text-faint ${pickable ? "cursor-pointer hover:text-ink" : ""}`}
          title={pickable ? "Click to select this line, Shift-click to extend, double-click for the whole change" : undefined}
          onClick={pickable ? (event) => event.detail === 1 && pick(index, event.shiftKey) : undefined}
          onDoubleClick={pickable ? () => pickRun(index) : undefined}
        >
          <span className="w-11 pr-2 text-right">{line.old ?? ""}</span>
          <span className="w-11 pr-2 text-right">{line.new ?? ""}</span>
          <span className={`w-4 text-center text-[12px] ${style.tone}`}>{style.sign}</span>
        </span>
        <Segments line={line.line} mark={style.mark} />
      </div>
    );
  };

  if (raw.hunks.length === 0) return <p className="p-4 text-[12.5px] text-slate">No textual differences.</p>;
  return (
    <div className="relative h-full">
      <div ref={scroller} className="h-full overflow-auto" onScroll={(event) => setViewport((current) => ({ ...current, top: event.currentTarget.scrollTop }))}>
        <div className="relative w-max min-w-full" style={{ height: total + bottomInset }}>
          <div className="absolute inset-x-0" style={{ top: offsets[first] ?? 0 }}>
            {raw.lines.slice(first, last + 1).map((line, index) => renderLine(line, first + index))}
          </div>
        </div>
      </div>
      {picked.size > 0 && (
        <div className="absolute right-4 top-10 z-10 flex items-center gap-2 rounded-panel border border-line-strong bg-raised py-1.5 pl-3 pr-1.5 text-[11.5px] shadow-float">
          <span className="tabular-nums text-soft">
            {picked.size} {picked.size === 1 ? "line" : "lines"} selected
          </span>
          {kind === "unstaged" && (
            <button type="button" onClick={() => discardLines(selection)} className={`${BUTTON} hover:border-del/60 hover:bg-del/10`}>
              Discard
            </button>
          )}
          <button type="button" onClick={() => void toggleLinesStage(selection)} className={`${BUTTON} border-add/50 hover:bg-add/10`}>
            {verb} Lines
          </button>
          <button type="button" onClick={() => setPicked(new Set())} className="px-1.5 text-slate hover:text-ink">
            Clear
          </button>
        </div>
      )}
    </div>
  );
}
