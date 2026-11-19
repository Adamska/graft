import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  ChangesetSummary,
  CommitDetails,
  DiffRow,
  DiffTarget,
  EngineMetrics,
  EntitySelector,
  FileDiff,
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

  analyzeChangeset: (target: DiffTarget) => invoke<ChangesetSummary>("analyze_changeset", { target }),
  fileDiff: (target: DiffTarget, path: string) => invoke<FileDiff>("file_diff", { target, path }),
  rawDiff: (target: DiffTarget, path: string) => invoke<DiffRow[]>("raw_diff", { target, path }),
  stagePaths: (paths: string[]) => invoke<void>("stage_paths", { paths }),
  unstagePaths: (paths: string[]) => invoke<void>("unstage_paths", { paths }),
  stageEntity: (path: string, selector: EntitySelector) => invoke<void>("stage_entity", { path, selector }),
  unstageEntity: (path: string, selector: EntitySelector) => invoke<void>("unstage_entity", { path, selector }),
  commitStaged: (message: string) => invoke<string>("commit_staged", { message }),

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
