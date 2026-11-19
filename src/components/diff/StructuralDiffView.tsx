import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import type { FileDiff } from "../../lib/types";
import { findingsFor, useAssistStore } from "../../store/useAssistStore";
import { useDiffStore, visibleBlocks } from "../../store/useDiffStore";
import { GUTTER_WIDTH, MoveRibbonOverlay } from "./MoveRibbonOverlay";
import { PaneRow, type RowActions, type Side } from "./PaneRows";
import { buildLayout, rowAt } from "./visualRows";

const OVERSCAN = 400;

/**
 * Side-by-side structural diff. Every block is flattened into fixed-height rows that
 * both panes render from the same list, so old and new code stay aligned no matter how
 * far a declaration moved; only the rows inside the viewport are mounted.
 */
export function StructuralDiffView({ file, bottomInset }: { file: FileDiff; bottomInset: number }) {
  const scroller = useRef<HTMLDivElement>(null);
  const [viewport, setViewport] = useState({ top: 0, height: 600, width: 800 });
  const target = useDiffStore((state) => state.target);
  const showMoved = useDiffStore((state) => state.showMoved);
  const expanded = useDiffStore((state) => state.expanded);
  const activeBlock = useDiffStore((state) => state.activeBlock);
  const setActiveBlock = useDiffStore((state) => state.setActiveBlock);
  const toggleExpanded = useDiffStore((state) => state.toggleExpanded);
  const toggleBlockStage = useDiffStore((state) => state.toggleBlockStage);
  const open = useDiffStore((state) => state.open);

  const findings = useAssistStore(useShallow((state) => findingsFor(state, target, file.path)));
  const layout = useMemo(() => buildLayout(visibleBlocks(file, showMoved), expanded, findings), [file, showMoved, expanded, findings]);
  const total = layout.offsets[layout.rows.length] ?? 0;
  const first = rowAt(layout.offsets, viewport.top - OVERSCAN);
  const last = rowAt(layout.offsets, viewport.top + viewport.height + OVERSCAN);
  const paneWidth = Math.max(0, (viewport.width - GUTTER_WIDTH) / 2);

  useLayoutEffect(() => {
    const element = scroller.current;
    if (!element) return;
    const measure = () => setViewport((current) => ({ ...current, height: element.clientHeight, width: element.clientWidth }));
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    scroller.current?.scrollTo({ top: 0 });
  }, [file.path]);

  /* Keep the block selected with J/K inside the viewport. */
  useEffect(() => {
    const element = scroller.current;
    const row = layout.headerRow.get(activeBlock);
    if (!element || row === undefined) return;
    const top = layout.offsets[row] ?? 0;
    if (top < element.scrollTop + 8) element.scrollTo({ top: Math.max(0, top - 24) });
    else if (top > element.scrollTop + element.clientHeight - bottomInset - 80) element.scrollTo({ top: top - element.clientHeight * 0.3 });
  }, [activeBlock, layout, bottomInset]);

  const actions: RowActions = useMemo(
    () => ({
      target: target?.kind ?? "commit",
      activeBlock,
      onActivate: setActiveBlock,
      onToggle: toggleExpanded,
      onStage: (block) => void toggleBlockStage(block),
      onOpenFile: (path) => target && void open(target, path),
    }),
    [target, activeBlock, setActiveBlock, toggleExpanded, toggleBlockStage, open],
  );

  const pane = (side: Side) => (
    <div
      className="no-scrollbar absolute top-0 overflow-x-auto overflow-y-hidden"
      style={{ [side === "old" ? "left" : "right"]: 0, width: paneWidth, height: total }}
    >
      <div className="w-max min-w-full" style={{ transform: `translateY(${layout.offsets[first] ?? 0}px)` }}>
        {layout.rows.slice(first, last + 1).map((row, index) => (
          <PaneRow key={first + index} row={row} side={side} width={paneWidth} actions={actions} />
        ))}
      </div>
    </div>
  );

  return (
    <div
      ref={scroller}
      className="relative h-full overflow-y-auto overflow-x-hidden"
      onScroll={(event) => {
        const top = event.currentTarget.scrollTop;
        setViewport((current) => (current.top === top ? current : { ...current, top }));
      }}
    >
      <div className="relative" style={{ height: total + bottomInset }}>
        {pane("old")}
        <div
          className="absolute top-0 border-x border-line bg-surface/60"
          style={{ left: `calc(50% - ${GUTTER_WIDTH / 2}px)`, width: GUTTER_WIDTH, height: total }}
        />
        {pane("new")}
        <MoveRibbonOverlay ribbons={layout.ribbons} scrollTop={viewport.top} height={viewport.height} activeBlock={activeBlock} />
      </div>
    </div>
  );
}
