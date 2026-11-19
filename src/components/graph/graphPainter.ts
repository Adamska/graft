import { formatAge } from "../../lib/format";
import type { GraphModel } from "../../lib/graphModel";
import type { RefEntry } from "../../lib/types";
import { paintEdges, widestLane } from "./graphEdges";
import { AGE_WIDTH, COLORS, FONTS, GRAPH_PAD, ROW_HEIGHT, laneColor, layoutFor } from "./graphTheme";

export interface GraphView {
  width: number;
  height: number;
  scrollTop: number;
  /** Eased lane count the graph column is currently sized for. */
  lanes: number;
}

export interface PaintState {
  model: GraphModel;
  /** Display order when a filter is active; `null` shows the full graph. */
  rowMap: Uint32Array | null;
  refs: RefEntry[];
  selectedRow: number;
  hoverIndex: number;
  now: number;
}

const LANE_EASING = 0.28;
const fitCache = new Map<string, string>();

export function displayCount(state: PaintState): number {
  return state.rowMap ? state.rowMap.length : state.model.total;
}

/**
 * Paints one frame and returns `true` while the graph column is still easing toward
 * the width its visible rails need, in which case the caller schedules another frame.
 */
export function paintGraph(ctx: CanvasRenderingContext2D, view: GraphView, state: PaintState): boolean {
  const { model, rowMap } = state;
  const count = displayCount(state);
  const first = Math.max(0, Math.floor(view.scrollTop / ROW_HEIGHT));
  const last = Math.min(count - 1, Math.floor((view.scrollTop + view.height) / ROW_HEIGHT));

  const wanted = rowMap ? 1 : widestLane(model, first - 1, last + 1) + 1;
  const gap = wanted - view.lanes;
  const settling = view.lanes > 0 && Math.abs(gap) > 0.02;
  view.lanes = settling ? view.lanes + gap * LANE_EASING : wanted;
  const layout = layoutFor(view.width, view.lanes);
  const centerY = (index: number) => index * ROW_HEIGHT - view.scrollTop + ROW_HEIGHT / 2;

  ctx.fillStyle = COLORS.background;
  ctx.fillRect(0, 0, view.width, view.height);

  for (let index = first; index <= last; index++) {
    const row = rowMap ? (rowMap[index] ?? 0) : index;
    const selected = row === state.selectedRow;
    if (!selected && index !== state.hoverIndex) continue;
    ctx.fillStyle = selected ? COLORS.selected : COLORS.hover;
    ctx.fillRect(0, centerY(index) - ROW_HEIGHT / 2, view.width, ROW_HEIGHT);
    if (selected) {
      ctx.fillStyle = COLORS.accent;
      ctx.fillRect(0, centerY(index) - ROW_HEIGHT / 2, 2, ROW_HEIGHT);
    }
  }

  if (!rowMap) paintEdges(ctx, view, model, layout, first - 1, last + 1);

  ctx.textBaseline = "middle";
  for (let index = first; index <= last; index++) {
    const row = rowMap ? (rowMap[index] ?? 0) : index;
    const x = GRAPH_PAD + (rowMap ? 0 : model.lane(row)) * layout.spacing;
    paintNode(ctx, x, centerY(index), laneColor(model.color(row)), model.parentCount(row) > 1);
    paintText(ctx, view, state, row, layout.graphWidth, centerY(index));
  }
  return settling;
}

function paintNode(ctx: CanvasRenderingContext2D, x: number, y: number, color: string, merge: boolean): void {
  ctx.beginPath();
  ctx.arc(x + 0.5, y, merge ? 3.5 : 4, 0, Math.PI * 2);
  ctx.fillStyle = merge ? COLORS.background : color;
  ctx.fill();
  ctx.lineWidth = merge ? 1.6 : 2;
  ctx.strokeStyle = merge ? color : COLORS.background;
  ctx.stroke();
}

function paintText(ctx: CanvasRenderingContext2D, view: GraphView, state: PaintState, row: number, left: number, y: number): void {
  const { model } = state;
  const right = view.width - AGE_WIDTH - 14;
  let x = left + 4;

  ctx.font = FONTS.pill;
  for (const refIndex of (model.refsByRow.get(row) ?? []).slice(0, 2)) {
    const ref = state.refs[refIndex];
    if (!ref || x > right - 60) break;
    const label = fit(ctx, ref.name, 104);
    const width = ctx.measureText(label).width + 10;
    const color = ref.kind === "tag" ? COLORS.tag : ref.kind === "remote" ? COLORS.slate : laneColor(model.color(row));
    ctx.beginPath();
    ctx.roundRect(x + 0.5, y - 8.5, width, 17, 3);
    if (ref.is_head) {
      ctx.fillStyle = color;
      ctx.fill();
    } else {
      ctx.strokeStyle = color;
      ctx.lineWidth = 1;
      ctx.stroke();
    }
    ctx.fillStyle = ref.is_head ? COLORS.background : color;
    ctx.fillText(label, x + 5.5, y + 0.5);
    x += width + 6;
  }

  ctx.font = FONTS.summary;
  ctx.fillStyle = row === state.selectedRow ? COLORS.ink : "#D5D7DC";
  ctx.fillText(fit(ctx, model.textOf(row).summary, right - x), x, y + 0.5);

  ctx.font = FONTS.age;
  ctx.fillStyle = COLORS.slate;
  ctx.textAlign = "right";
  ctx.fillText(formatAge(model.time(row), state.now), view.width - 14, y + 0.5);
  ctx.textAlign = "left";
}

/** Truncates with an ellipsis by bisecting on measured width; results are memoized. */
function fit(ctx: CanvasRenderingContext2D, text: string, maxWidth: number): string {
  if (maxWidth <= 8) return "";
  const key = `${ctx.font}|${Math.round(maxWidth)}|${text}`;
  const cached = fitCache.get(key);
  if (cached !== undefined) return cached;

  let fitted = text;
  if (ctx.measureText(text).width > maxWidth) {
    let [low, high] = [0, text.length];
    while (low < high) {
      const middle = (low + high + 1) >> 1;
      if (ctx.measureText(`${text.slice(0, middle)}…`).width <= maxWidth) low = middle;
      else high = middle - 1;
    }
    fitted = `${text.slice(0, low).trimEnd()}…`;
  }
  if (fitCache.size > 8000) fitCache.clear();
  fitCache.set(key, fitted);
  return fitted;
}
