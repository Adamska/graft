import { actions } from "../../lib/actions";
import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { BranchIcon, FolderIcon, SearchIcon, SidebarIcon } from "../ui/icons";
import { SyncBar } from "./SyncBar";

export function TitleBar() {
  const repo = useRepoStore((state) => state.repo);
  const pickRepository = useRepoStore((state) => state.pickRepository);
  const closeRepository = useRepoStore((state) => state.closeRepository);
  const toggleSidebar = useUiStore((state) => state.toggleSidebar);
  const setPaletteOpen = useUiStore((state) => state.setPaletteOpen);
  const head = repo?.head;

  return (
    <header data-tauri-drag-region className="flex h-10 shrink-0 items-center gap-3 border-b border-line bg-surface pl-[84px] pr-2.5 text-[12.5px]">
      {repo && head ? (
        <>
          <IconButton label="Toggle sidebar" onClick={toggleSidebar}>
            <SidebarIcon />
          </IconButton>
          <button type="button" onClick={closeRepository} className="shrink-0 font-semibold tracking-tight hover:text-accent" title="Back to start">
            {repo.name}
          </button>
          <button
            type="button"
            title={head.branch ? "Copy branch name" : "HEAD is detached: commits made here belong to no branch"}
            onClick={() => actions.copy(head.branch ?? head.oid ?? "", head.branch ? "branch name" : "commit hash")}
            className={`flex min-w-0 items-center gap-1.5 ${head.branch ? "text-slate hover:text-ink" : "text-edited"}`}
          >
            <BranchIcon className="shrink-0" />
            <span className="truncate">{head.branch ?? `detached at ${head.oid?.slice(0, 7) ?? "unborn"}`}</span>
          </button>
          <span className="mx-0.5 h-4 w-px shrink-0 bg-line-strong" />
          <SyncBar />
          <span data-tauri-drag-region className="min-w-2 flex-1 self-stretch" />
          <button
            type="button"
            onClick={() => setPaletteOpen(true)}
            className="flex h-7 w-[260px] shrink items-center gap-2 rounded-control border border-line bg-canvas/60 px-2.5 text-[12px] text-slate hover:border-line-strong hover:text-ink"
          >
            <SearchIcon width={12} height={12} className="shrink-0" />
            <span className="min-w-0 flex-1 truncate text-left">Search or run a command</span>
            <kbd className="font-mono text-[10.5px] text-faint">⌘K</kbd>
          </button>
          <IconButton label="Open another repository" onClick={() => void pickRepository()}>
            <FolderIcon />
          </IconButton>
        </>
      ) : (
        <span data-tauri-drag-region className="flex-1 text-slate">
          Graft
        </span>
      )}
    </header>
  );
}

function IconButton({ label, onClick, children }: { label: string; onClick: () => void; children: React.ReactNode }) {
  return (
    <button type="button" title={label} aria-label={label} onClick={onClick} className="flex size-7 shrink-0 items-center justify-center rounded-control text-slate hover:bg-hover hover:text-ink">
      {children}
    </button>
  );
}
