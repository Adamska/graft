export function App() {
  return (
    <div className="flex h-full flex-col bg-obsidian text-ink">
      <header data-tauri-drag-region className="h-9 shrink-0 border-b border-line" />
      <main className="flex flex-1 items-center justify-center">
        <div className="text-center">
          <h1 className="text-[15px] font-semibold tracking-tight">Graft</h1>
          <p className="mt-1 text-slate">Structural diffs for Git, at native speed.</p>
        </div>
      </main>
    </div>
  );
}
