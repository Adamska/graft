export const ROW_HEIGHT = 28;
export const LANE_WIDTH = 14;
export const GRAPH_PAD = 14;
export const AGE_WIDTH = 34;
export const SCROLLBAR_WIDTH = 10;

export const COLORS = {
  background: "#090A0D",
  ink: "#EDEDEF",
  slate: "#7E8594",
  faint: "#4D5361",
  hover: "rgba(255,255,255,0.035)",
  selected: "rgba(255,255,255,0.07)",
  accent: "#06B6D4",
  tag: "#D6B26E",
} as const;

/** Rail hues, kept clear of the diff accents so a lane never reads as add, delete, or move. */
export const LANE_COLORS = ["#5B9DF5", "#F2994A", "#2DD4BF", "#F472B6", "#A3E635", "#38BDF8", "#FB7185", "#D6B26E"] as const;

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
