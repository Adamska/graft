import { directory, fileName } from "../../lib/format";
import type { ChangeStatus, FileChange } from "../../lib/types";

const STATUS: Record<ChangeStatus, { letter: string; className: string }> = {
  added: { letter: "A", className: "text-add" },
  untracked: { letter: "U", className: "text-add" },
  modified: { letter: "M", className: "text-edited" },
  renamed: { letter: "R", className: "text-moved" },
  deleted: { letter: "D", className: "text-del" },
  conflicted: { letter: "!", className: "text-del" },
};

export interface FileListProps {
  files: FileChange[];
  selectedPath?: string | null;
  onSelect?: (file: FileChange) => void;
  /** Extra controls rendered at the right edge of each row. */
  trailing?: (file: FileChange) => React.ReactNode;
}

export function FileList({ files, selectedPath, onSelect, trailing }: FileListProps) {
  return (
    <ul>
      {files.map((file) => {
        const status = STATUS[file.status];
        const selected = file.path === selectedPath;
        return (
          <li key={file.path} className={`group flex h-7 items-center ${selected ? "bg-active" : "hover:bg-hover"}`}>
            <button
              type="button"
              onClick={() => onSelect?.(file)}
              title={file.old_path ? `${file.old_path} → ${file.path}` : file.path}
              className="flex h-full min-w-0 flex-1 items-center gap-2 pl-3.5 pr-2 text-left"
            >
              <span className={`w-3 shrink-0 text-center font-mono text-[11px] font-semibold ${status.className}`}>{status.letter}</span>
              <span className="shrink-0 text-[12.5px] text-ink">{fileName(file.path)}</span>
              <span className="min-w-0 truncate font-mono text-[11px] text-faint">{directory(file.path)}</span>
            </button>
            {trailing?.(file)}
          </li>
        );
      })}
    </ul>
  );
}
