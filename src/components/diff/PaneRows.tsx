import { memo } from "react";
import type { Block, DiffLine, DiffTarget, RiskFinding, RowKind, Severity } from "../../lib/types";
import { ChevronIcon } from "../ui/icons";
import { badgeFor, detailFor, ENTITY_LABELS, lineRange, qualifier, TONE_CLASSES, toneOf } from "./blockLabels";
import { ROW_HEIGHTS, type VisualRow } from "./visualRows";

export type Side = "old" | "new";

/** Below this pane width, headers drop qualifiers and secondary details. */
const COMPACT_WIDTH = 520;

export interface RowActions {
  target: DiffTarget["kind"];
  activeBlock: number;
  onActivate: (block: number) => void;
  onToggle: (block: number) => void;
  onStage: (block: Block) => void;
  onOpenFile: (path: string) => void;
}

const ROW_TINT: Record<Side, Partial<Record<RowKind, string>>> = {
  old: { del: "bg-del/[0.11]", change: "bg-del/[0.055]" },
  new: { add: "bg-add/[0.11]", change: "bg-add/[0.055]" },
};

const CodeLine = memo(function CodeLine({ line, side, kind }: { line: DiffLine | null; side: Side; kind: RowKind }) {
  if (!line) return <div className="void-cell h-5" />;
  const mark = side === "old" ? "mark-old" : "mark-new";
  return (
    <div className={`code-line flex h-5 ${ROW_TINT[side][kind] ?? ""}`}>
      <span className="sticky left-0 z-[1] w-11 shrink-0 select-none bg-obsidian pr-2.5 text-right text-[11px] text-faint">{line.number}</span>
      <span className="select-text pr-6">
        {line.segments.map(([text, tokenClass, changed], index) => (
          <span key={index} className={`tok-${tokenClass} ${changed ? mark : ""}`}>
            {text}
          </span>
        ))}
      </span>
    </div>
  );
});

function Header({ block, side, collapsed, compact, actions }: { block: Block; side: Side; collapsed: boolean; compact: boolean; actions: RowActions }) {
  const location = side === "old" ? block.old : block.new;
  const primary = side === "new" ? block.new !== null : block.new === null;
  const label = block.entity ? ENTITY_LABELS[block.entity] : "Lines";
  const collapsible = block.kind === "moved";
  const detail = detailFor(block);

  if (!location) return <div className="void-cell h-full border-t border-line" />;
  return (
    <div className="flex h-full items-center gap-2 border-t border-line bg-surface pl-3 pr-2">
      {primary && collapsible && (
        <button type="button" aria-label={collapsed ? "Expand block" : "Collapse block"} onClick={() => actions.onToggle(block.id)} className="-ml-1 text-slate hover:text-ink">
          <ChevronIcon open={!collapsed} width={11} height={11} />
        </button>
      )}
      <span className="shrink-0 text-[10.5px] uppercase tracking-[0.05em] text-faint">{label}</span>
      <span className="min-w-[72px] truncate font-mono text-[12px]" title={`${block.qualified} ${lineRange(location)}`}>
        {!compact && <span className="text-slate">{qualifier(block)}</span>}
        <span className="font-medium text-ink">{block.name || lineRange(location)}</span>
      </span>
      {!(compact && primary) && <span className="shrink-0 font-mono text-[10.5px] text-faint">{block.name ? lineRange(location) : ""}</span>}
      {primary && (
        <>
          <span className={`ml-auto shrink-0 border px-1.5 font-mono text-[10.5px] leading-[17px] ${TONE_CLASSES[toneOf(block.kind)]}`} title={detail ?? undefined}>
            {badgeFor(block)}
          </span>
          {detail && !compact && <span className="shrink-0 font-mono text-[10.5px] text-slate">{detail}</span>}
          {block.stageable && actions.target !== "commit" && (
            <button
              type="button"
              onClick={(event) => {
                event.stopPropagation();
                actions.onStage(block);
              }}
              className="shrink-0 border border-line-strong px-2 text-[11px] leading-[18px] text-ink hover:border-add/60 hover:bg-add/10"
            >
              {actions.target === "unstaged" ? "Stage" : "Unstage"} {label}
            </button>
          )}
        </>
      )}
    </div>
  );
}

function Origin({ block, side, actions }: { block: Block; side: Side; actions: RowActions }) {
  if (side === "new") return <div className="void-cell h-full" />;
  return (
    <div className="flex h-full items-center gap-2 border border-dashed border-moved/35 bg-moved/[0.04] pl-3 pr-2">
      <span className="size-1.5 shrink-0 rounded-full bg-moved" />
      <span className="min-w-0 truncate font-mono text-[12px] text-[#C9CCD3]">
        <span className="text-slate">{qualifier(block)}</span>
        {block.name}
      </span>
      <span className="shrink-0 font-mono text-[10.5px] text-faint">{lineRange(block.old)}</span>
      {block.new?.path ? (
        <button type="button" onClick={() => actions.onOpenFile(block.new?.path ?? "")} className="ml-auto shrink-0 font-mono text-[10.5px] text-moved hover:underline">
          {badgeFor(block)}
        </button>
      ) : (
        <span className="ml-auto shrink-0 font-mono text-[10.5px] text-moved">{badgeFor(block)}</span>
      )}
      {block.stageable && block.link === null && actions.target !== "commit" && (
        <button type="button" onClick={() => actions.onStage(block)} className="shrink-0 border border-line-strong px-2 text-[11px] leading-[18px] hover:border-add/60 hover:bg-add/10">
          {actions.target === "unstaged" ? "Stage" : "Unstage"} Removal
        </button>
      )}
    </div>
  );
}

const SEVERITY: Record<Severity, string> = { high: "bg-del", medium: "bg-edited", low: "bg-slate" };

function Risk({ finding, shown }: { finding: RiskFinding; shown: boolean }) {
  if (!shown) return <div className="void-cell h-full" />;
  return (
    <div className="flex h-full items-center gap-2 border-l-2 border-edited/70 bg-edited/[0.06] pl-3 pr-2 text-[11.5px]" title={`${finding.detail} (${finding.source})`}>
      <span className={`size-1.5 shrink-0 rounded-full ${SEVERITY[finding.severity]}`} />
      <span className="shrink-0 font-medium text-ink">{finding.title}</span>
      <span className="min-w-0 truncate text-slate">{finding.detail}</span>
    </div>
  );
}

/** One row of one pane; both panes render the same row list so they stay aligned. */
export const PaneRow = memo(function PaneRow({ row, side, width, actions }: { row: VisualRow; side: Side; width: number; actions: RowActions }) {
  const height = ROW_HEIGHTS[row.kind];
  if (row.kind === "gap") return <div style={{ height }} />;

  const active = row.block.id === actions.activeBlock;
  const activate = () => actions.onActivate(row.block.id);
  if (row.kind === "code") {
    return (
      <div onPointerDown={activate}>
        <CodeLine line={side === "old" ? row.row.old : row.row.new} side={side} kind={row.row.kind} />
      </div>
    );
  }

  /* Chrome rows stay pinned to the pane's left edge while code scrolls sideways. */
  return (
    <div style={{ height }} onPointerDown={activate}>
      <div className="sticky left-0 h-full" style={{ width }}>
        {row.kind === "header" && (
          <div className={`relative h-full ${active ? "brightness-125" : ""}`}>
            {active && side === "old" && <span className="absolute inset-y-0 left-0 z-[2] w-0.5 bg-moved" />}
            <Header block={row.block} side={side} collapsed={row.collapsed} compact={width < COMPACT_WIDTH} actions={actions} />
          </div>
        )}
        {row.kind === "origin" && <Origin block={row.block} side={side} actions={actions} />}
        {row.kind === "risk" && <Risk finding={row.finding} shown={side === (row.block.new ? "new" : "old")} />}
        {row.kind === "fold" && (
          <div className="flex h-full items-center bg-white/[0.02] pl-11 font-mono text-[10.5px] text-faint">
            {side === "old" ? `⋯ ${row.hidden} unchanged ${row.hidden === 1 ? "line" : "lines"}` : ""}
          </div>
        )}
      </div>
    </div>
  );
});
