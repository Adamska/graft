export const ROW_HEIGHT = 28;
export const LANE_WIDTH = 14;
export const GRAPH_PAD = 14;
export const AGE_WIDTH = 34;
export const SCROLLBAR_WIDTH = 10;

/** Canvas counterparts of the CSS theme tokens in `index.css`. */
export const COLORS = {
  background: "#1B1D23",
  ink: "#DCDFE6",
  soft: "#B7BCC8",
  slate: "#8A91A0",
  faint: "#5F6674",
  hover: "rgba(255,255,255,0.04)",
  selected: "rgba(255,255,255,0.08)",
  thumb: "rgba(255,255,255,0.13)",
  thumbActive: "rgba(255,255,255,0.26)",
  accent: "#7AA7EC",
  tag: "#D2B273",
} as const;

/** Rail hues, kept clear of the diff accents so a lane never reads as add, delete, or move. */
export const LANE_COLORS = ["#6E9FE8", "#E0A062", "#56C2AE", "#DE8CB7", "#A6C766", "#68BCE4", "#E58991", "#D2B273"] as const;

export const FONTS = {
  summary: '12.5px "Inter Variable", system-ui, sans-serif',
  age: '11px "Inter Variable", system-ui, sans-serif',
  pill: '500 10.5px "Inter Variable", system-ui, sans-serif',
} as const;

export interface GraphLayout {
  spacing: number;
  graphWidth: number;
}

/**
 * Sizes the graph column for a (possibly fractional, mid-easing) lane count. Lanes share
 * at most 42% of the pane; beyond that the rails compress instead of pushing text out.
 */
export function layoutFor(width: number, laneCount: number): GraphLayout {
  const gaps = Math.max(0, laneCount - 1);
  const budget = Math.max(GRAPH_PAD * 2 + LANE_WIDTH, width * 0.42);
  const spacing = gaps > 0 ? Math.min(LANE_WIDTH, (budget - GRAPH_PAD * 2) / gaps) : LANE_WIDTH;
  return { spacing, graphWidth: GRAPH_PAD * 2 + spacing * gaps };
}

export function laneColor(index: number): string {
  return LANE_COLORS[index % LANE_COLORS.length] ?? LANE_COLORS[0];
}
