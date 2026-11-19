import { Channel, invoke } from "@tauri-apps/api/core";
import type { Blame, ConflictSide, LineSelection, MergeMode, OperationStep, PullMode, ResetMode, Tracking } from "./actionTypes";
import type {
  AssistantSettings,
  AssistantStatus,
  ChangesetSummary,
  CommitDetails,
  CommitDraft,
  DiffRow,
  DiffTarget,
  EngineMetrics,
  EntitySelector,
  FileDiff,
  GraphStats,
  Operation,
  RecentRepository,
  RepoSnapshot,
  RiskReport,
  WorkingStatus,
} from "./types";

export const ipc = {
  openRepository: (path: string) => invoke<RepoSnapshot>("open_repository", { path }),
  refreshRepository: () => invoke<RepoSnapshot>("refresh_repository"),
  recentRepositories: () => invoke<RecentRepository[]>("recent_repositories"),
  launchPath: () => invoke<string | null>("launch_path"),
  commitDetails: (oid: string) => invoke<CommitDetails>("commit_details", { oid }),
  workingStatus: () => invoke<WorkingStatus>("working_status"),
  engineMetrics: () => invoke<EngineMetrics>("engine_metrics"),

  analyzeChangeset: (target: DiffTarget) => invoke<ChangesetSummary>("analyze_changeset", { target }),
  fileDiff: (target: DiffTarget, path: string) => invoke<FileDiff>("file_diff", { target, path }),
  rawDiff: (target: DiffTarget, path: string) => invoke<DiffRow[]>("raw_diff", { target, path }),
  blameFile: (target: DiffTarget, path: string) => invoke<Blame>("blame_file", { target, path }),
  stagePaths: (paths: string[]) => invoke<void>("stage_paths", { paths }),
  unstagePaths: (paths: string[]) => invoke<void>("unstage_paths", { paths }),
  discardPaths: (paths: string[]) => invoke<void>("discard_paths", { paths }),
  stageEntity: (path: string, selector: EntitySelector) => invoke<void>("stage_entity", { path, selector }),
  unstageEntity: (path: string, selector: EntitySelector) => invoke<void>("unstage_entity", { path, selector }),
  stageLines: (path: string, selection: LineSelection) => invoke<void>("stage_lines", { path, selection }),
  unstageLines: (path: string, selection: LineSelection) => invoke<void>("unstage_lines", { path, selection }),
  discardLines: (path: string, selection: LineSelection) => invoke<void>("discard_lines", { path, selection }),
  commitStaged: (message: string, amend: boolean) => invoke<string>("commit_staged", { message, amend }),
  headMessage: () => invoke<string>("head_message"),

  checkout: (revision: string, detach: boolean) => invoke<void>("checkout", { revision, detach }),
  trackBranch: (name: string) => invoke<string>("track_branch", { name }),
  createBranch: (name: string, start: string | null, checkout: boolean) => invoke<void>("create_branch", { name, start, checkout }),
  renameBranch: (from: string, to: string) => invoke<void>("rename_branch", { from, to }),
  deleteBranch: (name: string, force: boolean, remote: boolean) => invoke<void>("delete_branch", { name, force, remote }),
  createTag: (name: string, target: string, message: string | null) => invoke<void>("create_tag", { name, target, message }),
  deleteTag: (name: string) => invoke<void>("delete_tag", { name }),
  stashSave: (message: string, includeUntracked: boolean) => invoke<void>("stash_save", { message, includeUntracked }),
  stashApply: (index: number, pop: boolean) => invoke<void>("stash_apply", { index, pop }),
  stashDrop: (index: number) => invoke<void>("stash_drop", { index }),

  branchTracking: () => invoke<Tracking[]>("branch_tracking"),
  fetch: () => invoke<string>("fetch"),
  pull: (mode: PullMode) => invoke<string>("pull", { mode }),
  push: (branch: string | null, force: boolean) => invoke<string>("push", { branch, force }),
  pushTag: (name: string) => invoke<string>("push_tag", { name }),
  merge: (revision: string, mode: MergeMode) => invoke<void>("merge", { revision, mode }),
  rebase: (onto: string) => invoke<void>("rebase", { onto }),
  cherryPick: (oid: string) => invoke<void>("cherry_pick", { oid }),
  revert: (oid: string) => invoke<void>("revert", { oid }),
  reset: (revision: string, mode: ResetMode) => invoke<void>("reset", { revision, mode }),
  advanceOperation: (operation: Operation, step: OperationStep) => invoke<void>("advance_operation", { operation, step }),
  resolveConflict: (path: string, side: ConflictSide) => invoke<void>("resolve_conflict", { path, side }),

  generateCommitMessage: () => invoke<CommitDraft>("generate_commit_message"),
  reviewRisks: (target: DiffTarget) => invoke<RiskReport>("review_risks", { target }),
  assistantStatus: () => invoke<AssistantStatus>("assistant_status"),
  saveAssistantSettings: (settings: AssistantSettings) => invoke<AssistantStatus>("save_assistant_settings", { settings }),

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

  /** Graph rows of the commits that touched `path`, newest first. */
  async fileHistory(path: string): Promise<Uint32Array> {
    return new Uint32Array(await invoke<ArrayBuffer>("file_history", { path }));
  },
};

export function errorMessage(error: unknown): string {
  return typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
}
