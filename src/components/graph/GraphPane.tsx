import { useEffect, useRef, useState } from "react";
import { formatCount } from "../../lib/format";
import { graphStore, useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { SearchIcon } from "../ui/icons";
import { CommitGraphCanvas } from "./CommitGraphCanvas";

const FILTER_DEBOUNCE_MILLIS = 70;

export function GraphPane() {
  const width = useUiStore((state) => state.graphWidth);
  const focused = useUiStore((state) => state.focus === "graph");
  const filterRows = useRepoStore((state) => state.filterRows);
  useRepoStore((state) => state.graphVersion);

  return (
    <section
      className="relative flex shrink-0 flex-col border-r border-line"
      style={{ width }}
      data-focused={focused}
      aria-label="Commit graph"
    >
      <FilterBar matches={filterRows?.length ?? null} total={graphStore.model.total} />
      <WorkingTreeRow />
      <div className="min-h-0 flex-1">
        <CommitGraphCanvas />
      </div>
      <ResizeHandle />
      {focused && <span className="pointer-events-none absolute inset-x-0 top-0 h-px bg-accent/70" />}
    </section>
  );
}

function FilterBar({ matches, total }: { matches: number | null; total: number }) {
  const setFilter = useRepoStore((state) => state.setFilter);
  const [query, setQuery] = useState("");
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const timer = setTimeout(() => void setFilter(query), FILTER_DEBOUNCE_MILLIS);
    return () => clearTimeout(timer);
  }, [query, setFilter]);

  return (
    <label className="flex h-9 shrink-0 items-center gap-2 border-b border-line px-3 text-slate focus-within:text-ink">
      <SearchIcon />
      <input
        ref={input}
        data-graph-filter
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onKeyDown={(event) => {
          if (event.key !== "Escape") return;
          setQuery("");
          input.current?.blur();
        }}
        placeholder="Filter commits by message, author, or hash"
        spellCheck={false}
        className="min-w-0 flex-1 bg-transparent text-[12.5px] text-ink outline-none placeholder:text-faint"
      />
      <span className="shrink-0 text-[11px] tabular-nums text-faint">
        {matches === null ? formatCount(total) : `${formatCount(matches)} of ${formatCount(total)}`}
      </span>
    </label>
  );
}

function WorkingTreeRow() {
  const status = useRepoStore((state) => state.status);
  const selected = useRepoStore((state) => state.selection.kind === "working");
  const selectWorking = useRepoStore((state) => state.selectWorking);
  const [staged, pending] = [status.staged.length, status.unstaged.length];

  return (
    <button
      type="button"
      onClick={() => {
        useUiStore.getState().setFocus("graph");
        selectWorking();
      }}
      className={`relative flex h-8 shrink-0 items-center gap-2.5 border-b border-line pl-[9px] pr-3.5 text-left text-[12.5px] ${
        selected ? "bg-active text-ink" : "text-soft hover:bg-hover"
      }`}
    >
      {selected && <span className="absolute inset-y-0 left-0 w-0.5 bg-accent" />}
      <span className="size-[9px] rounded-full border-[1.5px] border-dashed border-slate" />
      <span className="flex-1 font-medium">Working tree</span>
      {staged + pending === 0 ? (
        <span className="text-[11px] text-faint">clean</span>
      ) : (
        <span className="flex gap-2 text-[11px] tabular-nums">
          {staged > 0 && <span className="text-add">{staged} staged</span>}
          {pending > 0 && <span className="text-edited">{pending} changed</span>}
        </span>
      )}
    </button>
  );
}

function ResizeHandle() {
  const setGraphWidth = useUiStore((state) => state.setGraphWidth);
  const onPointerDown = (event: React.PointerEvent<HTMLDivElement>) => {
    const origin = event.clientX;
    const initial = useUiStore.getState().graphWidth;
    const move = (moved: PointerEvent) => setGraphWidth(initial + moved.clientX - origin);
    const stop = () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", stop);
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", stop);
  };
  return (
    <div
      role="separator"
      aria-orientation="vertical"
      onPointerDown={onPointerDown}
      className="absolute inset-y-0 -right-[3px] z-10 w-[5px] cursor-col-resize hover:bg-accent/40"
    />
  );
}
