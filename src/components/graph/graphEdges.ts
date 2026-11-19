import { CHUNK_SHIFT, type GraphModel } from "../../lib/graphModel";
import { GRAPH_PAD, LANE_COLORS, ROW_HEIGHT, type GraphLayout, laneColor } from "./graphTheme";

/** `lower` is -1 for an edge whose parent has not been streamed yet. */
type EdgeVisitor = (upper: number, lower: number, lane: number, color: number) => void;

/**
 * Edges are bucketed by the row they end on, so the set crossing rows `from..=to` is
 * every bucket from `from` to the end of the chunk holding `to`, plus that chunk's
 * still-open edges. The scan is bounded by one chunk regardless of history size.
 */
export function forEachVisibleEdge(model: GraphModel, from: number, to: number, visit: EdgeVisitor): void {
  const top = Math.max(0, from);
  const lastChunk = Math.min(model.chunks.length - 1, Math.max(0, to) >> CHUNK_SHIFT);
  for (let index = top >> CHUNK_SHIFT; index <= lastChunk; index++) {
    const chunk = model.chunks[index];
    if (!chunk) continue;
    for (let local = Math.max(top - chunk.firstRow, 0); local < chunk.rowCount; local++) {
      for (let edge = chunk.edgeEnds[local] ?? 0; edge < (chunk.edgeEnds[local + 1] ?? 0); edge++) {
        const upper = chunk.edges[edge * 2] ?? 0;
        const packed = chunk.edges[edge * 2 + 1] ?? 0;
        if (upper <= to) visit(upper, chunk.firstRow + local, packed & 0xffff, (packed >>> 16) & 0xff);
      }
    }
    if (index !== lastChunk) continue;
    for (let edge = 0; edge < chunk.openEdges.length; edge += 2) {
      const upper = chunk.openEdges[edge] ?? 0;
      const packed = chunk.openEdges[edge + 1] ?? 0;
      if (upper <= to) visit(upper, -1, packed & 0xffff, (packed >>> 16) & 0xff);
    }
  }
}

/** Highest lane index any node or rail occupies between rows `from` and `to`. */
export function widestLane(model: GraphModel, from: number, to: number): number {
  let widest = 0;
  for (let row = Math.max(0, from); row <= Math.min(model.total - 1, to); row++) widest = Math.max(widest, model.lane(row));
  forEachVisibleEdge(model, from, to, (_upper, _lower, lane) => {
    widest = Math.max(widest, lane);
  });
  return widest;
}

interface EdgeView {
  height: number;
  scrollTop: number;
}

/** Strokes every visible rail, batched into one Path2D per lane color. */
export function paintEdges(ctx: CanvasRenderingContext2D, view: EdgeView, model: GraphModel, layout: GraphLayout, from: number, to: number): void {
  const paths = LANE_COLORS.map(() => new Path2D());
  const x = (lane: number) => GRAPH_PAD + lane * layout.spacing + 0.5;
  const y = (row: number) => row * ROW_HEIGHT - view.scrollTop + ROW_HEIGHT / 2;

  forEachVisibleEdge(model, from, to, (upper, lower, lane, color) => {
    const path = paths[color % paths.length];
    if (!path) return;
    const travel = x(lane);
    const [startX, startY] = [x(model.lane(upper)), y(upper)];
    path.moveTo(startX, startY);
    if (lower < 0) {
      bend(path, startX, startY, travel, startY + ROW_HEIGHT);
      path.lineTo(travel, view.height + ROW_HEIGHT);
      return;
    }
    const [endX, endY] = [x(model.lane(lower)), y(lower)];
    if (lower - upper === 1) return bend(path, startX, startY, endX, endY);
    bend(path, startX, startY, travel, startY + ROW_HEIGHT);
    path.lineTo(travel, endY - ROW_HEIGHT);
    bend(path, travel, endY - ROW_HEIGHT, endX, endY);
  });

  ctx.save();
  ctx.beginPath();
  ctx.rect(0, 0, layout.graphWidth + 2, view.height);
  ctx.clip();
  ctx.lineWidth = 1.5;
  ctx.lineCap = "round";
  paths.forEach((path, color) => {
    ctx.strokeStyle = laneColor(color);
    ctx.stroke(path);
  });
  ctx.restore();
}

/** Cubic with vertical tangents at both ends, so a rail leaves and enters nodes head-on. */
function bend(path: Path2D, fromX: number, fromY: number, toX: number, toY: number): void {
  if (fromX === toX) return path.lineTo(toX, toY);
  const reach = (toY - fromY) * 0.62;
  path.bezierCurveTo(fromX, fromY + reach, toX, toY - reach, toX, toY);
}
