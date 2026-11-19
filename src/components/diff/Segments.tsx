import { memo } from "react";
import type { DiffLine } from "../../lib/types";

/** Syntax-colored text of one line; `mark` is the class applied to its changed runs. */
export const Segments = memo(function Segments({ line, mark = "" }: { line: DiffLine; mark?: string }) {
  return (
    <span className="select-text pr-6">
      {line.segments.map(([text, tokenClass, changed], index) => (
        <span key={index} className={`tok-${tokenClass} ${changed ? mark : ""}`}>
          {text}
        </span>
      ))}
    </span>
  );
});
