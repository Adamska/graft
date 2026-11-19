import logo from "../../assets/logo.png";
import { formatAge } from "../../lib/format";
import { useRepoStore } from "../../store/useRepoStore";
import { FolderIcon } from "../ui/icons";

export function WelcomeScreen({ dragging }: { dragging: boolean }) {
  const opening = useRepoStore((state) => state.phase === "opening");
  const recents = useRepoStore((state) => state.recents);
  const pickRepository = useRepoStore((state) => state.pickRepository);
  const openRepository = useRepoStore((state) => state.openRepository);
  const openDemo = useRepoStore((state) => state.openDemo);
  const now = Date.now() / 1000;

  return (
    <main className="flex min-h-0 flex-1 items-center justify-center overflow-y-auto px-8">
      <div className={`w-full max-w-[520px] rounded-panel border px-9 py-9 transition-colors ${dragging ? "border-accent bg-accent/[0.04]" : "border-transparent"}`}>
        <div className="flex items-center gap-3.5">
          <img src={logo} alt="" width={52} height={52} draggable={false} className="-ml-1 select-none" />
          <h1 className="text-[22px] font-semibold tracking-[-0.02em]">Graft</h1>
        </div>
        <p className="mt-3 max-w-[400px] text-[13.5px] leading-relaxed text-slate">
          A native Git client that diffs syntax trees instead of lines. Moved functions stay moved, and only real edits light up.
        </p>

        <div className="mt-7 flex flex-col gap-2">
          <Action label="Open Repository" hint="⌘O" disabled={opening} onClick={() => void pickRepository()} primary />
          <Action label="Open Built-In Refactor Demo Repo" hint="20 commits · 5 branches" disabled={opening} onClick={() => void openDemo()} />
        </div>
        <p className="mt-3 text-[12px] text-faint">{opening ? "Opening repository…" : "Or drop a repository folder anywhere in this window."}</p>

        {recents.length > 0 && (
          <section className="mt-9">
            <h2 className="mb-1.5 text-[11px] font-medium uppercase tracking-[0.06em] text-slate">Recent</h2>
            <ul className="border-t border-line">
              {recents.slice(0, 6).map((recent) => (
                <li key={recent.path} className="border-b border-line">
                  <button
                    type="button"
                    disabled={opening}
                    onClick={() => void openRepository(recent.path)}
                    className="flex h-9 w-full items-center gap-2.5 px-1 text-left hover:bg-hover"
                  >
                    <FolderIcon className="shrink-0 text-slate" />
                    <span className="shrink-0 text-[13px] font-medium">{recent.name}</span>
                    <span className="min-w-0 flex-1 truncate font-mono text-[11px] text-faint" dir="rtl">
                      {recent.path}
                    </span>
                    <span className="shrink-0 text-[11px] tabular-nums text-faint">{formatAge(recent.opened_at, now)}</span>
                  </button>
                </li>
              ))}
            </ul>
          </section>
        )}
      </div>
    </main>
  );
}

function Action(props: { label: string; hint: string; primary?: boolean; disabled: boolean; onClick: () => void }) {
  return (
    <button
      type="button"
      disabled={props.disabled}
      onClick={props.onClick}
      className={`flex h-10 items-center justify-between rounded-control border px-3.5 text-[13px] font-medium disabled:opacity-50 ${
        props.primary
          ? "border-transparent bg-accent text-canvas hover:bg-accent/85"
          : "border-line-strong text-ink hover:border-accent/60 hover:bg-accent/[0.05]"
      }`}
    >
      {props.label}
      <span className={`font-mono text-[11px] font-normal ${props.primary ? "text-canvas/60" : "text-slate"}`}>{props.hint}</span>
    </button>
  );
}
