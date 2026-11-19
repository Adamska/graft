import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  CommitDetails,
  EngineMetrics,
  GraphStats,
  RecentRepository,
  RepoSnapshot,
  WorkingStatus,
} from "./types";

export const ipc = {
  openRepository: (path: string) => invoke<RepoSnapshot>("open_repository", { path }),
  refreshRepository: () => invoke<RepoSnapshot>("refresh_repository"),
  recentRepositories: () => invoke<RecentRepository[]>("recent_repositories"),
  createDemoRepository: () => invoke<string>("create_demo_repository"),
  commitDetails: (oid: string) => invoke<CommitDetails>("commit_details", { oid }),
  workingStatus: () => invoke<WorkingStatus>("working_status"),
  engineMetrics: () => invoke<EngineMetrics>("engine_metrics"),

  /** Streams binary graph chunks; resolves once the whole history has been sent. */
  streamGraph(onChunk: (frame: ArrayBuffer) => void): Promise<GraphStats> {
    const channel = new Channel<ArrayBuffer>();
    channel.onmessage = onChunk;
    return invoke<GraphStats>("stream_graph", { onChunk: channel });
  },

  async searchCommits(query: string, limit?: number): Promise<Uint32Array> {
    const bytes = await invoke<ArrayBuffer>("search_commits", { query, limit });
    return new Uint32Array(bytes);
  },
};

export function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}
