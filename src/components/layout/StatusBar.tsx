import { useEffect, useState } from "react";
import { formatCount, formatDuration, formatMegabytes } from "../../lib/format";
import { graphStore, useRepoStore } from "../../store/useRepoStore";
import { useDiffStore } from "../../store/useDiffStore";
import { frameStats } from "../graph/CommitGraphCanvas";

const POLL_MILLIS = 2000;

/** Live engine telemetry: every figure is measured, none are estimates. */
export function StatusBar() {
  const ready = useRepoStore((state) => state.phase === "ready");
  const metrics = useRepoStore((state) => state.metrics);
  const refreshMetrics = useRepoStore((state) => state.refreshMetrics);
  useRepoStore((state) => state.graphVersion);
  const astMicros = useDiffStore((state) => state.astMicros);
  const [frameMillis, setFrameMillis] = useState(0);

  useEffect(() => {
    const poll = () => {
      void refreshMetrics();
      setFrameMillis(frameStats.millis);
    };
    poll();
    const timer = setInterval(poll, POLL_MILLIS);
    return () => clearInterval(timer);
  }, [refreshMetrics]);

  const model = graphStore.model;
  const indexed = `${formatCount(model.total)} commits ${model.complete ? "indexed" : "streaming"} in ${formatDuration(model.elapsedMicros)}`;
  const items = [
    ready && model.total > 0 ? indexed : null,
    ready && astMicros !== null ? `AST Diff: ${formatDuration(astMicros)}` : null,
    ready && frameMillis > 0 ? `Frame: ${frameMillis.toFixed(1)}ms` : null,
    metrics ? `RAM: ${formatMegabytes(metrics.resident_bytes)}` : null,
    metrics ? `${metrics.threads} threads` : null,
  ].filter((item): item is string => item !== null);

  return (
    <footer className="flex h-6 shrink-0 items-center justify-end gap-2 border-t border-line bg-surface px-3 font-mono text-[10.5px] tabular-nums text-slate">
      {items.map((item, index) => (
        <span key={item} className="flex items-center gap-2">
          {index > 0 && <span className="text-faint">·</span>}
          {item}
        </span>
      ))}
    </footer>
  );
}
