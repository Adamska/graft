import { type ReactNode, useState } from "react";
import type { Tracking } from "../../lib/actionTypes";
import { formatAge } from "../../lib/format";
import { refActions } from "../../lib/refActions";
import type { RefEntry, RefKind } from "../../lib/types";
import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { activateRef, refMenu, stashMenu } from "../menus/refMenus";
import { BranchIcon, ChevronIcon, FolderIcon, PlusIcon, RemoteIcon, StashIcon, TagIcon } from "../ui/icons";

const ICONS: Record<RefKind, ReactNode> = { local: <BranchIcon />, remote: <RemoteIcon />, tag: <TagIcon /> };

/** `2↑ 1↓` for a branch that differs from its upstream, `gone` when the upstream was deleted. */
function trackingLabel(tracking: Tracking | undefined): string | undefined {
  if (!tracking) return undefined;
  if (tracking.gone) return "gone";
  const parts = [tracking.ahead > 0 ? `${tracking.ahead}↑` : "", tracking.behind > 0 ? `${tracking.behind}↓` : ""];
  return parts.filter(Boolean).join(" ") || undefined;
}

export function Sidebar() {
  const repo = useRepoStore((state) => state.repo);
  const recents = useRepoStore((state) => state.recents);
  const tracking = useRepoStore((state) => state.tracking);
  const selection = useRepoStore((state) => state.selection);
  const selectRef = useRepoStore((state) => state.selectRef);
  const selectOid = useRepoStore((state) => state.selectOid);
  const openRepository = useRepoStore((state) => state.openRepository);
  const openMenu = useUiStore((state) => state.openMenu);
  if (!repo) return null;

  const selectedOid = selection.kind === "commit" ? selection.oid : null;
  const group = (kind: RefKind) =>
    repo.refs
      .map((ref, index) => ({ ref, index }))
      .filter((entry) => entry.ref.kind === kind)
      .map(({ ref, index }) => {
        const upstream = kind === "local" ? tracking.find((entry) => entry.branch === ref.name) : undefined;
        return <RefRow key={`${kind}:${ref.name}`} entry={ref} tracking={upstream} active={ref.oid === selectedOid} onSelect={() => selectRef(index)} onMenu={(event) => openMenu(event, refMenu(ref, upstream))} />;
      });
  const [locals, remotes, tags] = [group("local"), group("remote"), group("tag")];
  const others = recents.filter((recent) => recent.path !== repo.path);

  return (
    <nav className="flex w-60 shrink-0 flex-col overflow-y-auto border-r border-line bg-surface py-1.5" aria-label="References">
      <Section title="Branches" count={locals.length} action={{ label: "New branch", run: () => refActions.createBranch() }}>
        {locals}
      </Section>
      <Section title="Remotes" count={remotes.length} initiallyOpen={remotes.length <= 12}>
        {remotes}
      </Section>
      <Section title="Tags" count={tags.length} initiallyOpen={tags.length <= 12}>
        {tags}
      </Section>
      <Section title="Stashes" count={repo.stashes.length} action={{ label: "Stash changes", run: () => refActions.stash() }}>
        {repo.stashes.map((stash) => (
          <Row
            key={stash.oid}
            icon={<StashIcon />}
            label={stash.message.replace(/^(WIP on|On) [^:]+: /, "")}
            title={`${stash.message}\nDouble-click to pop`}
            active={stash.oid === selectedOid}
            onClick={() => void selectOid(stash.oid)}
            onActivate={() => void refActions.applyStash(stash, true)}
            onMenu={(event) => openMenu(event, stashMenu(stash))}
          />
        ))}
      </Section>
      <Section title="Recent" count={others.length}>
        {others.map((recent) => (
          <Row key={recent.path} icon={<FolderIcon />} label={recent.name} title={recent.path} trailing={formatAge(recent.opened_at, Date.now() / 1000)} onClick={() => void openRepository(recent.path)} />
        ))}
      </Section>
    </nav>
  );
}

function Section(props: { title: string; count: number; initiallyOpen?: boolean; action?: { label: string; run: () => void }; children: ReactNode }) {
  const [open, setOpen] = useState(props.initiallyOpen ?? true);
  return (
    <section className="group/section mb-1">
      <div className="flex h-7 items-center pr-2 text-[11px] font-medium uppercase tracking-[0.06em] text-slate">
        <button type="button" onClick={() => setOpen(!open)} className="flex h-full min-w-0 flex-1 items-center gap-1.5 pl-2.5 uppercase tracking-[0.06em] hover:text-ink">
          <ChevronIcon open={open} width={10} height={10} />
          <span className="flex-1 text-left">{props.title}</span>
        </button>
        {props.action && (
          <button type="button" title={props.action.label} aria-label={props.action.label} onClick={props.action.run} className="mr-1 hidden size-5 items-center justify-center rounded-control hover:bg-active hover:text-ink group-hover/section:flex">
            <PlusIcon width={12} height={12} />
          </button>
        )}
        <span className="tabular-nums text-faint">{props.count}</span>
      </div>
      {open && (props.count > 0 ? props.children : <p className="px-7 pb-1 text-[12px] text-faint">None</p>)}
    </section>
  );
}

interface RefRowProps {
  entry: RefEntry;
  tracking: Tracking | undefined;
  active: boolean;
  onSelect: () => void;
  onMenu: (event: React.MouseEvent) => void;
}

function RefRow({ entry, tracking, active, onSelect, onMenu }: RefRowProps) {
  const hint = entry.kind === "tag" ? "Double-click to check out detached" : entry.is_head ? "Checked out" : "Double-click to check out";
  return (
    <Row
      icon={ICONS[entry.kind]}
      label={entry.name}
      title={`${entry.name} at ${entry.oid.slice(0, 7)}${tracking ? `, tracking ${tracking.upstream}` : ""}\n${hint}`}
      active={active}
      emphasized={entry.is_head}
      trailing={trackingLabel(tracking)}
      trailingTone={tracking?.gone ? "text-del" : "text-accent"}
      onClick={onSelect}
      onActivate={() => activateRef(entry)}
      onMenu={onMenu}
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
  trailingTone?: string;
  onClick?: () => void;
  onActivate?: () => void;
  onMenu?: (event: React.MouseEvent) => void;
}

function Row({ icon, label, title, active, emphasized, trailing, trailingTone = "text-faint", onClick, onActivate, onMenu }: RowProps) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      onDoubleClick={onActivate}
      onContextMenu={onMenu}
      className={`flex h-7 w-full items-center gap-2 pl-7 pr-2.5 text-left text-[12.5px] ${active ? "bg-active text-ink" : "text-soft hover:bg-hover"}`}
    >
      <span className={emphasized ? "text-add" : "text-slate"}>{icon}</span>
      <span className={`min-w-0 flex-1 truncate ${emphasized ? "font-medium text-ink" : ""}`}>{label}</span>
      {trailing && <span className={`shrink-0 text-[10.5px] tabular-nums ${trailingTone}`}>{trailing}</span>}
    </button>
  );
}
