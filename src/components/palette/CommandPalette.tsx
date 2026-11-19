import { useEffect, useMemo, useRef, useState } from "react";
import { fuzzyMatch } from "../../lib/fuzzy";
import { useUiStore } from "../../store/useUiStore";
import { SearchIcon } from "../ui/icons";
import { buildItems, type PaletteItem, searchCommitItems } from "./paletteItems";

const RESULT_LIMIT = 60;
const COMMIT_LIMIT = 30;
const MUTATION_PENALTY = 12;

interface Ranked {
  item: PaletteItem;
  positions: number[];
}

/** Cmd+K palette: fuzzy-searches actions, branches, tags, changed files, and commits. */
export function CommandPalette() {
  const open = useUiStore((state) => state.paletteOpen);
  return open ? <Palette /> : null;
}

function Palette() {
  const close = () => useUiStore.getState().setPaletteOpen(false);
  const [query, setQuery] = useState("");
  const [cursor, setCursor] = useState(0);
  const [commits, setCommits] = useState<PaletteItem[]>([]);
  const list = useRef<HTMLUListElement>(null);
  const items = useMemo(buildItems, []);

  useEffect(() => {
    let current = true;
    void searchCommitItems(query, COMMIT_LIMIT).then((found) => current && setCommits(found));
    return () => {
      current = false;
    };
  }, [query]);

  const ranked = useMemo(() => {
    const scored: (Ranked & { score: number })[] = [];
    for (const item of items) {
      const match = fuzzyMatch(query, item.label);
      if (match) scored.push({ item, positions: match.positions, score: match.score - (item.mutating ? MUTATION_PENALTY : 0) });
    }
    if (query.trim() !== "") scored.sort((left, right) => right.score - left.score);
    const matchedCommits = commits.map((item) => ({ item, positions: fuzzyMatch(query, item.label)?.positions ?? [] }));
    return [...scored, ...matchedCommits].slice(0, RESULT_LIMIT);
  }, [items, commits, query]);

  useEffect(() => setCursor(0), [query]);
  useEffect(() => {
    list.current?.children[cursor]?.scrollIntoView({ block: "nearest" });
  }, [cursor]);

  const run = (entry: Ranked | undefined) => {
    if (!entry) return;
    close();
    void entry.item.run();
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "ArrowDown" || (event.ctrlKey && event.key === "n")) setCursor((index) => Math.min(ranked.length - 1, index + 1));
    else if (event.key === "ArrowUp" || (event.ctrlKey && event.key === "p")) setCursor((index) => Math.max(0, index - 1));
    else if (event.key === "Enter") run(ranked[cursor]);
    else if (event.key === "Escape") close();
    else return;
    event.preventDefault();
    event.stopPropagation();
  };

  return (
    <div className="fixed inset-0 z-40 flex justify-center bg-black/35 pt-[14vh]" onPointerDown={close}>
      <div
        role="dialog"
        aria-label="Command palette"
        className="flex h-fit max-h-[62vh] w-[600px] flex-col border border-line-strong bg-raised shadow-float"
        onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <label className="flex h-11 shrink-0 items-center gap-2.5 border-b border-line px-3.5 text-slate">
          <SearchIcon />
          <input
            autoFocus
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Search commits, branches, files, or run a command"
            spellCheck={false}
            className="min-w-0 flex-1 bg-transparent text-[13.5px] text-ink outline-none placeholder:text-faint"
          />
        </label>
        <ul ref={list} className="min-h-0 flex-1 overflow-y-auto py-1" role="listbox">
          {ranked.map((entry, index) => (
            <li
              key={entry.item.id}
              role="option"
              aria-selected={index === cursor}
              onPointerMove={() => setCursor(index)}
              onClick={() => run(entry)}
              className={`flex h-8 items-center gap-3 px-3.5 text-[13px] ${index === cursor ? "bg-active text-ink" : "text-soft"}`}
            >
              <span className="w-12 shrink-0 text-[10.5px] uppercase tracking-[0.05em] text-faint">{entry.item.group}</span>
              <span className={`min-w-0 flex-1 truncate ${entry.item.group === "File" ? "font-mono text-[12px]" : ""}`}>
                <Highlighted text={entry.item.label} positions={entry.positions} />
              </span>
              {entry.item.hint && <span className="shrink-0 font-mono text-[10.5px] text-slate">{entry.item.hint}</span>}
            </li>
          ))}
          {ranked.length === 0 && <li className="px-3.5 py-3 text-[12.5px] text-slate">Nothing matches “{query}”.</li>}
        </ul>
        <footer className="flex h-7 shrink-0 items-center gap-4 border-t border-line px-3.5 font-mono text-[10.5px] text-faint">
          <span>↑↓ navigate</span>
          <span>↵ run</span>
          <span>esc close</span>
        </footer>
      </div>
    </div>
  );
}

function Highlighted({ text, positions }: { text: string; positions: number[] }) {
  if (positions.length === 0) return <>{text}</>;
  const marked = new Set(positions);
  return (
    <>
      {Array.from(text, (character, index) =>
        marked.has(index) ? (
          <span key={index} className="text-accent">
            {character}
          </span>
        ) : (
          character
        ),
      )}
    </>
  );
}
