import { useLayoutEffect, useMemo, useRef, useState } from "react";
import type { Blame, BlameHunk } from "../../lib/actionTypes";
import { formatAge, formatTimestamp, shortOid } from "../../lib/format";
import { useRepoStore } from "../../store/useRepoStore";
import { Segments } from "./Segments";

const LINE_HEIGHT = 20;
const OVERSCAN_LINES = 24;
const ANNOTATION_WIDTH = 248;

const isCommitted = (hunk: BlameHunk) => !/^0+$/.test(hunk.oid);

/**
 * The file with the commit that last touched each line. Runs of lines from one commit
 * share a single annotation; the bar beside it is brighter the more recent the commit.
 */
export function BlameView({ blame, bottomInset }: { blame: Blame; bottomInset: number }) {
  const scroller = useRef<HTMLDivElement>(null);
  const [viewport, setViewport] = useState({ top: 0, height: 600 });
  const [hovered, setHovered] = useState(-1);
  const selectOid = useRepoStore((state) => state.selectOid);
  const now = Date.now() / 1000;

  const { owners, oldest, span } = useMemo(() => {
    const owners = new Int32Array(blame.lines.length).fill(-1);
    blame.hunks.forEach((hunk, index) => owners.fill(index, hunk.start - 1, hunk.start - 1 + hunk.count));
    const times = blame.hunks.filter(isCommitted).map((hunk) => hunk.time);
    const oldest = Math.min(...times);
    return { owners, oldest, span: Math.max(1, Math.max(...times) - oldest) };
  }, [blame]);

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
  const last = Math.min(blame.lines.length, Math.ceil((viewport.top + viewport.height) / LINE_HEIGHT) + OVERSCAN_LINES);

  if (blame.lines.length === 0) return <p className="p-4 text-[12.5px] text-slate">Blame is not available for this file at this revision.</p>;
  return (
    <div ref={scroller} className="h-full overflow-auto" onScroll={(event) => setViewport((current) => ({ ...current, top: event.currentTarget.scrollTop }))} onPointerLeave={() => setHovered(-1)}>
      <div className="relative w-max min-w-full" style={{ height: blame.lines.length * LINE_HEIGHT + bottomInset }}>
        <div className="absolute inset-x-0" style={{ top: first * LINE_HEIGHT }}>
          {blame.lines.slice(first, last).map((line, offset) => {
            const index = first + offset;
            const owner = owners[index] ?? -1;
            const hunk = blame.hunks[owner];
            const committed = hunk !== undefined && isCommitted(hunk);
            const leads = hunk !== undefined && hunk.start - 1 === index;
            const heat = committed ? 0.18 + (0.82 * (hunk.time - oldest)) / span : 0;
            return (
              <div key={index} className={`code-line flex h-5 ${owner === hovered ? "bg-hover" : ""}`} onPointerEnter={() => setHovered(owner)}>
                <button
                  type="button"
                  disabled={!committed}
                  title={committed ? `${hunk.summary}\n${hunk.author}, ${formatTimestamp(hunk.time)}\nClick to show this commit` : "Not committed yet"}
                  onClick={() => hunk && void selectOid(hunk.oid)}
                  style={{ width: ANNOTATION_WIDTH }}
                  className={`sticky left-0 z-[1] flex h-5 shrink-0 items-center gap-2 bg-surface pr-2.5 text-left font-sans text-[11px] ${leads ? "border-t border-line" : ""} ${owner === hovered && committed ? "text-ink" : "text-slate"}`}
                >
                  <span className="h-full w-[3px] shrink-0 bg-accent" style={{ opacity: heat }} />
                  {leads && hunk && (
                    <>
                      <span className="shrink-0 font-mono text-[10.5px] text-accent">{committed ? shortOid(hunk.oid) : "·······"}</span>
                      <span className="min-w-0 flex-1 truncate">{committed ? hunk.summary : "Not committed yet"}</span>
                      {committed && <span className="shrink-0 tabular-nums text-faint">{formatAge(hunk.time, now)}</span>}
                    </>
                  )}
                </button>
                <span className="w-11 shrink-0 select-none pr-2.5 text-right text-[11px] text-faint">{line.number}</span>
                <Segments line={line} />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
