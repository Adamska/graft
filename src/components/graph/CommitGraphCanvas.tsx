import { useCallback, useEffect, useRef } from "react";
import { graphStore, useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { displayCount, paintGraph, type PaintState } from "./graphPainter";
import { COLORS, ROW_HEIGHT, SCROLLBAR_WIDTH } from "./graphTheme";

/** Duration of the most recent paint, read by the status bar. */
export const frameStats = { millis: 0 };

const MIN_THUMB = 28;

interface Viewport {
  width: number;
  height: number;
  scrollTop: number;
  lanes: number;
  hoverIndex: number;
  dragOffset: number | null;
}

/**
 * Virtualized commit graph on a single canvas. Scrolling is driven by wheel deltas
 * rather than a native scroller: there is no spacer element to hit browser height
 * limits at a million rows, and every frame is painted from the same state it scrolls.
 */
export function CommitGraphCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const viewport = useRef<Viewport>({ width: 0, height: 0, scrollTop: 0, lanes: 0, hoverIndex: -1, dragOffset: null });
  const frame = useRef(0);
  const paintRef = useRef<() => void>(() => undefined);

  const graphVersion = useRepoStore((state) => state.graphVersion);
  const selection = useRepoStore((state) => state.selection);
  const rowMap = useRepoStore((state) => state.filterRows);
  const refs = useRepoStore((state) => state.repo?.refs);
  const selectedRow = selection.kind === "commit" ? selection.row : -1;

  const snapshot = useCallback((): PaintState => {
    const now = Date.now() / 1000;
    return { model: graphStore.model, rowMap, refs: refs ?? [], selectedRow, hoverIndex: viewport.current.hoverIndex, now };
  }, [rowMap, refs, selectedRow]);

  const maxScroll = useCallback(
    () => Math.max(0, displayCount(snapshot()) * ROW_HEIGHT - viewport.current.height),
    [snapshot],
  );

  const paint = useCallback(() => {
    frame.current = 0;
    const canvas = canvasRef.current;
    const ctx = canvas?.getContext("2d", { alpha: false });
    if (!canvas || !ctx) return;
    const view = viewport.current;
    const started = performance.now();
    const ratio = window.devicePixelRatio || 1;
    view.scrollTop = Math.min(view.scrollTop, maxScroll());

    ctx.setTransform(ratio, 0, 0, ratio, 0, 0);
    const settling = paintGraph(ctx, view, snapshot());
    const limit = maxScroll();
    if (limit > 0) {
      const thumb = Math.max(MIN_THUMB, (view.height * view.height) / (limit + view.height));
      ctx.fillStyle = view.dragOffset === null ? COLORS.thumb : COLORS.thumbActive;
      ctx.beginPath();
      ctx.roundRect(view.width - 7, (view.scrollTop / limit) * (view.height - thumb) + 2, 4, thumb - 4, 2);
      ctx.fill();
    }
    frameStats.millis = performance.now() - started;
    if (settling) frame.current = requestAnimationFrame(() => paintRef.current());
  }, [maxScroll, snapshot]);
  paintRef.current = paint;

  const schedule = useCallback(() => {
    if (frame.current === 0) frame.current = requestAnimationFrame(paint);
  }, [paint]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const resize = () => {
      const bounds = canvas.getBoundingClientRect();
      const ratio = window.devicePixelRatio || 1;
      viewport.current.width = bounds.width;
      viewport.current.height = bounds.height;
      canvas.width = Math.round(bounds.width * ratio);
      canvas.height = Math.round(bounds.height * ratio);
      schedule();
    };
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    void document.fonts.ready.then(schedule);

    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const delta = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? event.deltaY * ROW_HEIGHT : event.deltaY;
      viewport.current.scrollTop = Math.max(0, Math.min(maxScroll(), viewport.current.scrollTop + delta));
      schedule();
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      observer.disconnect();
      canvas.removeEventListener("wheel", onWheel);
      cancelAnimationFrame(frame.current);
      frame.current = 0;
    };
  }, [maxScroll, schedule]);

  useEffect(schedule, [schedule, graphVersion]);

  useEffect(() => {
    const view = viewport.current;
    const index = rowMap ? rowMap.indexOf(selectedRow) : selectedRow;
    if (index < 0 || view.height === 0) return;
    const top = index * ROW_HEIGHT;
    if (top < view.scrollTop) view.scrollTop = Math.max(0, top - ROW_HEIGHT * 2);
    else if (top + ROW_HEIGHT > view.scrollTop + view.height) view.scrollTop = top - view.height + ROW_HEIGHT * 3;
    schedule();
  }, [rowMap, selectedRow, schedule]);

  const scrollToPointer = (clientY: number, offset: number) => {
    const view = viewport.current;
    const limit = maxScroll();
    const thumb = Math.max(MIN_THUMB, (view.height * view.height) / (limit + view.height));
    const bounds = canvasRef.current?.getBoundingClientRect();
    const position = (clientY - (bounds?.top ?? 0) - offset) / Math.max(1, view.height - thumb);
    view.scrollTop = Math.max(0, Math.min(limit, position * limit));
    schedule();
  };

  const onPointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const view = viewport.current;
    const bounds = event.currentTarget.getBoundingClientRect();
    const [x, y] = [event.clientX - bounds.left, event.clientY - bounds.top];
    useUiStore.getState().setFocus("graph");

    if (x >= view.width - SCROLLBAR_WIDTH && maxScroll() > 0) {
      const limit = maxScroll();
      const thumb = Math.max(MIN_THUMB, (view.height * view.height) / (limit + view.height));
      const thumbTop = (view.scrollTop / limit) * (view.height - thumb);
      view.dragOffset = y >= thumbTop && y <= thumbTop + thumb ? y - thumbTop : thumb / 2;
      event.currentTarget.setPointerCapture(event.pointerId);
      scrollToPointer(event.clientY, view.dragOffset);
      return;
    }
    const index = Math.floor((y + view.scrollTop) / ROW_HEIGHT);
    if (index >= 0 && index < displayCount(snapshot())) {
      useRepoStore.getState().selectCommit(rowMap ? (rowMap[index] ?? 0) : index);
    }
  };

  const onPointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const view = viewport.current;
    if (view.dragOffset !== null) return scrollToPointer(event.clientY, view.dragOffset);
    const y = event.clientY - event.currentTarget.getBoundingClientRect().top;
    const index = Math.floor((y + view.scrollTop) / ROW_HEIGHT);
    if (index !== view.hoverIndex) {
      view.hoverIndex = index;
      schedule();
    }
  };

  const release = () => {
    viewport.current.dragOffset = null;
    viewport.current.hoverIndex = -1;
    schedule();
  };

  return (
    <canvas
      ref={canvasRef}
      className="block h-full w-full"
      style={{ background: COLORS.background }}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={() => (viewport.current.dragOffset = null)}
      onPointerLeave={release}
    />
  );
}
