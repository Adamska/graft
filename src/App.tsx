import { useEffect } from "react";
import { NoticeToast } from "./components/layout/NoticeToast";
import { StatusBar } from "./components/layout/StatusBar";
import { TitleBar } from "./components/layout/TitleBar";
import { Workspace } from "./components/layout/Workspace";
import { AssistantSettingsDialog } from "./components/palette/AssistantSettingsDialog";
import { CommandPalette } from "./components/palette/CommandPalette";
import { WelcomeScreen } from "./components/welcome/WelcomeScreen";
import { useDropTarget } from "./lib/useDropTarget";
import { useGlobalKeys } from "./lib/useGlobalKeys";
import { useRepoStore } from "./store/useRepoStore";

export function App() {
  const ready = useRepoStore((state) => state.phase === "ready");
  const loadRecents = useRepoStore((state) => state.loadRecents);
  const openRepository = useRepoStore((state) => state.openRepository);
  const dragging = useDropTarget(openRepository);
  useGlobalKeys();

  useEffect(() => void loadRecents(), [loadRecents]);

  return (
    <div className={`flex h-full flex-col bg-obsidian text-ink ${dragging && ready ? "outline outline-1 -outline-offset-1 outline-moved" : ""}`}>
      <TitleBar />
      {ready ? <Workspace /> : <WelcomeScreen dragging={dragging} />}
      <StatusBar />
      <CommandPalette />
      <AssistantSettingsDialog />
      <NoticeToast />
    </div>
  );
}
