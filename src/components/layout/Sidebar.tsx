import { type ReactNode, useState } from "react";
import { formatAge } from "../../lib/format";
import type { RefEntry, RefKind } from "../../lib/types";
import { useRepoStore } from "../../store/useRepoStore";
import { BranchIcon, ChevronIcon, FolderIcon, RemoteIcon, StashIcon, TagIcon } from "../ui/icons";

const ICONS: Record<RefKind, ReactNode> = { local: <BranchIcon />, remote: <RemoteIcon />, tag: <TagIcon /> };

export function Sidebar() {
  const repo = useRepoStore((state) => state.repo);
  const recents = useRepoStore((state) => state.recents);
  const selection = useRepoStore((state) => state.selection);
  const selectRef = useRepoStore((state) => state.selectRef);
  const openRepository = useRepoStore((state) => state.openRepository);
  if (!repo) return null;

  const selectedOid = selection.kind === "commit" ? selection.oid : null;
  const group = (kind: RefKind) =>
    repo.refs
      .map((ref, index) => ({ ref, index }))
      .filter((entry) => entry.ref.kind === kind)
      .map(({ ref, index }) => (
        <RefRow key={`${kind}:${ref.name}`} entry={ref} active={ref.oid === selectedOid} onSelect={() => selectRef(index)} />
      ));
  const [locals, remotes, tags] = [group("local"), group("remote"), group("tag")];
  const others = recents.filter((recent) => recent.path !== repo.path);

  return (
    <nav className="flex w-60 shrink-0 flex-col overflow-y-auto border-r border-line bg-surface py-1.5" aria-label="References">
      <Section title="Branches" count={locals.length}>
        {locals}
      </Section>
      <Section title="Remotes" count={remotes.length} initiallyOpen={remotes.length <= 12}>
        {remotes}
      </Section>
      <Section title="Tags" count={tags.length} initiallyOpen={tags.length <= 12}>
        {tags}
      </Section>
      <Section title="Stashes" count={repo.stashes.length}>
        {repo.stashes.map((stash) => (
          <Row key={stash.oid} icon={<StashIcon />} label={stash.message} title={stash.message} />
        ))}
      </Section>
      <Section title="Recent" count={others.length}>
        {others.map((recent) => (
          <Row
            key={recent.path}
            icon={<FolderIcon />}
            label={recent.name}
            title={recent.path}
            trailing={formatAge(recent.opened_at, Date.now() / 1000)}
            onClick={() => void openRepository(recent.path)}
          />
        ))}
      </Section>
    </nav>
  );
}

function Section(props: { title: string; count: number; initiallyOpen?: boolean; children: ReactNode }) {
  const [open, setOpen] = useState(props.initiallyOpen ?? true);
  return (
    <section className="mb-1">
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className="flex h-7 w-full items-center gap-1.5 px-2.5 text-[11px] font-medium uppercase tracking-[0.06em] text-slate hover:text-ink"
      >
        <ChevronIcon open={open} width={10} height={10} />
        <span className="flex-1 text-left">{props.title}</span>
        <span className="tabular-nums text-faint">{props.count}</span>
      </button>
      {open && (props.count > 0 ? props.children : <p className="px-7 pb-1 text-[12px] text-faint">None</p>)}
    </section>
  );
}

function RefRow({ entry, active, onSelect }: { entry: RefEntry; active: boolean; onSelect: () => void }) {
  return (
    <Row
      icon={ICONS[entry.kind]}
      label={entry.name}
      title={`${entry.name} at ${entry.oid.slice(0, 7)}`}
      active={active}
      emphasized={entry.is_head}
      trailing={entry.is_head ? "HEAD" : undefined}
      onClick={onSelect}
    />
  );
}

interface RowProps {
  icon: ReactNode;
  label: string;
  title: string;
  active?: boolean;
  emphasized?: boolean;
  trailing?: string;
  onClick?: () => void;
}

function Row({ icon, label, title, active, emphasized, trailing, onClick }: RowProps) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={`flex h-7 w-full items-center gap-2 pl-7 pr-2.5 text-left text-[12.5px] ${
        active ? "bg-white/[0.07] text-ink" : "text-[#C9CCD3] hover:bg-white/[0.035]"
      }`}
    >
      <span className={emphasized ? "text-add" : "text-slate"}>{icon}</span>
      <span className={`min-w-0 flex-1 truncate ${emphasized ? "font-medium text-ink" : ""}`}>{label}</span>
      {trailing && <span className="shrink-0 text-[10.5px] tabular-nums text-faint">{trailing}</span>}
    </button>
  );
}
