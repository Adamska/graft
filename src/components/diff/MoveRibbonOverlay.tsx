import type { Ribbon } from "./visualRows";

export const GUTTER_WIDTH = 44;

const TONES = {
  moved: { fill: "rgba(6,182,212,0.13)", stroke: "rgba(6,182,212,0.75)" },
  edited: { fill: "rgba(245,158,11,0.13)", stroke: "rgba(245,158,11,0.8)" },
} as const;

interface Props {
  ribbons: Ribbon[];
  scrollTop: number;
  height: number;
  activeBlock: number;
}

/**
 * Bezier ribbons across the center gutter, from the placeholder where a declaration
 * used to live to the block it moved to. The SVG only spans the viewport, so ribbons
 * whose far end is thousands of pixels away are simply clipped at its edge.
 */
export function MoveRibbonOverlay({ ribbons, scrollTop, height, activeBlock }: Props) {
  const visible = ribbons.filter((ribbon) => {
    const top = Math.min(ribbon.fromTop, ribbon.toTop);
    const bottom = Math.max(ribbon.fromBottom, ribbon.blockBottom);
    return bottom >= scrollTop && top <= scrollTop + height;
  });

  return (
    <svg
      className="pointer-events-none absolute z-10"
      style={{ left: `calc(50% - ${GUTTER_WIDTH / 2}px)`, top: scrollTop }}
      width={GUTTER_WIDTH}
      height={height}
      aria-hidden="true"
    >
      {visible.map((ribbon) => {
        const tone = ribbon.edited ? TONES.edited : TONES.moved;
        const [fromTop, fromBottom] = [ribbon.fromTop - scrollTop + 4, ribbon.fromBottom - scrollTop - 4];
        const [toTop, toBottom] = [ribbon.toTop - scrollTop + 5, ribbon.toBottom - scrollTop - 5];
        const middle = GUTTER_WIDTH / 2;
        const band = [
          `M0 ${fromTop}`,
          `C${middle} ${fromTop} ${middle} ${toTop} ${GUTTER_WIDTH} ${toTop}`,
          `L${GUTTER_WIDTH} ${toBottom}`,
          `C${middle} ${toBottom} ${middle} ${fromBottom} 0 ${fromBottom}Z`,
        ].join(" ");
        const edges = [
          `M0 ${fromTop} C${middle} ${fromTop} ${middle} ${toTop} ${GUTTER_WIDTH} ${toTop}`,
          `M0 ${fromBottom} C${middle} ${fromBottom} ${middle} ${toBottom} ${GUTTER_WIDTH} ${toBottom}`,
        ].join(" ");
        const emphasis = ribbon.block === activeBlock ? 1 : 0.6;
        return (
          <g key={ribbon.block} opacity={emphasis}>
            <path d={band} fill={tone.fill} />
            <path d={edges} fill="none" stroke={tone.stroke} strokeWidth={1} />
            <rect x={GUTTER_WIDTH - 2} y={toBottom + 5} width={2} height={Math.max(0, ribbon.blockBottom - scrollTop - toBottom - 5)} fill={tone.stroke} opacity={0.5} />
          </g>
        );
      })}
    </svg>
  );
}
