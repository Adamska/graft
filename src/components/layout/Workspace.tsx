import { useUiStore } from "../../store/useUiStore";
import { GraphPane } from "../graph/GraphPane";
import { InspectorPane } from "../inspector/InspectorPane";
import { Sidebar } from "./Sidebar";

/** Three-pane workspace: references, commit graph, and the change inspector. */
export function Workspace() {
  const sidebarOpen = useUiStore((state) => state.sidebarOpen);
  return (
    <div className="flex min-h-0 flex-1">
      {sidebarOpen && <Sidebar />}
      <GraphPane />
      <InspectorPane />
    </div>
  );
}
