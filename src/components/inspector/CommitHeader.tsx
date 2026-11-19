import { formatTimestamp, shortOid } from "../../lib/format";
import type { CommitDetails } from "../../lib/types";

export function CommitHeader({ details }: { details: CommitDetails }) {
  return (
    <header className="shrink-0 select-text border-b border-line px-4 py-3">
      <h2 className="text-[14px] font-semibold leading-snug tracking-[-0.005em]">{details.summary}</h2>
      {details.body && <p className="mt-1.5 max-h-24 overflow-y-auto whitespace-pre-wrap text-[12.5px] leading-relaxed text-slate">{details.body}</p>}
      <dl className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11.5px] text-slate">
        <dd className="text-[#C9CCD3]">{details.author}</dd>
        <dd>{formatTimestamp(details.time)}</dd>
        <dd className="font-mono text-moved">{shortOid(details.oid)}</dd>
        {details.parents.length > 0 && (
          <dd className="font-mono text-faint">
            {details.parents.length > 1 ? "merge of " : "parent "}
            {details.parents.map(shortOid).join(" + ")}
          </dd>
        )}
      </dl>
    </header>
  );
}
