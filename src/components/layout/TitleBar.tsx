import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { BranchIcon, FolderIcon, SearchIcon, SidebarIcon } from "../ui/icons";

export function TitleBar() {
  const repo = useRepoStore((state) => state.repo);
  const pickRepository = useRepoStore((state) => state.pickRepository);
  const closeRepository = useRepoStore((state) => state.closeRepository);
  const toggleSidebar = useUiStore((state) => state.toggleSidebar);
  const setPaletteOpen = useUiStore((state) => state.setPaletteOpen);

  return (
    <header
      data-tauri-drag-region
      className="flex h-9 shrink-0 items-center gap-3 border-b border-line bg-surface pl-[84px] pr-2 text-[12.5px]"
    >
      {repo ? (
        <>
          <IconButton label="Toggle sidebar" onClick={toggleSidebar}>
            <SidebarIcon />
          </IconButton>
          <button type="button" onClick={closeRepository} className="font-semibold tracking-tight hover:text-moved" title="Back to start">
            {repo.name}
          </button>
          <span className="flex items-center gap-1.5 text-slate">
            <BranchIcon />
            {repo.head.branch ?? `detached at ${repo.head.oid?.slice(0, 7) ?? "unborn"}`}
          </span>
          <span data-tauri-drag-region className="flex-1 self-stretch" />
          <button
            type="button"
            onClick={() => setPaletteOpen(true)}
            className="flex h-6 w-[280px] items-center gap-2 border border-line px-2 text-[12px] text-slate hover:border-line-strong hover:text-ink"
          >
            <SearchIcon width={12} height={12} />
            <span className="flex-1 text-left">Search or run a command</span>
            <kbd className="font-mono text-[10.5px] text-faint">⌘K</kbd>
          </button>
          <span data-tauri-drag-region className="flex-1 self-stretch" />
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
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className="flex size-6 items-center justify-center rounded-[4px] text-slate hover:bg-white/[0.06] hover:text-ink"
    >
      {children}
    </button>
  );
}
