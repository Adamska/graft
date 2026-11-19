import { useLayoutEffect, useMemo, useRef, useState } from "react";
import type { DiffLine, DiffRow } from "../../lib/types";

const LINE_HEIGHT = 20;
const OVERSCAN_LINES = 20;

type RawLine =
  | { kind: "context" | "add" | "del"; old: number | null; new: number | null; line: DiffLine }
  | { kind: "fold"; hidden: number };

const STYLES = {
  context: { sign: " ", row: "", mark: "" },
  add: { sign: "+", row: "bg-add/[0.11]", mark: "mark-new" },
  del: { sign: "-", row: "bg-del/[0.11]", mark: "mark-old" },
} as const;

/** Turns aligned rows into a unified sequence: a run of changed pairs becomes its deletions, then its additions. */
function unify(rows: DiffRow[]): RawLine[] {
  const lines: RawLine[] = [];
  let additions: RawLine[] = [];
  const flush = () => {
    lines.push(...additions);
    additions = [];
  };
  for (const row of rows) {
    if (row.kind !== "change") flush();
    if (row.kind === "fold") lines.push({ kind: "fold", hidden: row.hidden });
    else if (row.kind === "context" && row.new) lines.push({ kind: "context", old: row.old?.number ?? null, new: row.new.number, line: row.new });
    else {
      if (row.old) lines.push({ kind: "del", old: row.old.number, new: null, line: row.old });
      if (row.new) additions.push({ kind: "add", old: null, new: row.new.number, line: row.new });
    }
  }
  flush();
  return lines;
}

/** Classic unified line diff (Myers), virtualized by fixed line height. */
export function RawDiffView({ rows, bottomInset }: { rows: DiffRow[]; bottomInset: number }) {
  const scroller = useRef<HTMLDivElement>(null);
  const [viewport, setViewport] = useState({ top: 0, height: 600 });
  const lines = useMemo(() => unify(rows), [rows]);

  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element) return;
    const measure = () => setViewport((current) => ({ ...current, height: element.clientHeight }));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  const first = Math.max(0, Math.floor(viewport.top / LINE_HEIGHT) - OVERSCAN_LINES);
  const last = Math.min(lines.length, Math.ceil((viewport.top + viewport.height) / LINE_HEIGHT) + OVERSCAN_LINES);

  if (lines.length === 0) return <p className="p-4 text-[12.5px] text-slate">No textual differences.</p>;
  return (
    <div ref={scroller} className="h-full overflow-auto" onScroll={(event) => setViewport((current) => ({ ...current, top: event.currentTarget.scrollTop }))}>
      <div className="relative w-max min-w-full" style={{ height: lines.length * LINE_HEIGHT + bottomInset }}>
        <div className="absolute inset-x-0" style={{ top: first * LINE_HEIGHT }}>
          {lines.slice(first, last).map((line, index) =>
            line.kind === "fold" ? (
              <div key={first + index} className="code-line h-5 bg-moved/[0.05] pl-[104px] text-[10.5px] text-slate">
                @@ {line.hidden} unchanged {line.hidden === 1 ? "line" : "lines"} @@
              </div>
            ) : (
              <div key={first + index} className={`code-line flex h-5 ${STYLES[line.kind].row}`}>
                <span className="w-11 shrink-0 select-none pr-2 text-right text-[11px] text-faint">{line.old ?? ""}</span>
                <span className="w-11 shrink-0 select-none pr-2 text-right text-[11px] text-faint">{line.new ?? ""}</span>
                <span className={`w-4 shrink-0 select-none text-center ${line.kind === "add" ? "text-add" : line.kind === "del" ? "text-del" : "text-faint"}`}>
                  {STYLES[line.kind].sign}
                </span>
                <span className="select-text pr-6">
                  {line.line.segments.map(([text, tokenClass, changed], segment) => (
                    <span key={segment} className={`tok-${tokenClass} ${changed ? STYLES[line.kind].mark : ""}`}>
                      {text}
                    </span>
                  ))}
                </span>
              </div>
            ),
          )}
        </div>
      </div>
    </div>
  );
}
