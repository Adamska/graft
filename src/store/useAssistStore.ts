import { create } from "zustand";
import { errorMessage, ipc } from "../lib/ipc";
import type { AssistantSettings, AssistantStatus, DiffTarget, RiskFinding } from "../lib/types";
import { useDiffStore } from "./useDiffStore";
import { useUiStore } from "./useUiStore";

interface AssistState {
  commitMessage: string;
  /** Where the current draft came from: a model name, or `built-in rules`. */
  draftSource: string | null;
  generating: boolean;
  findings: RiskFinding[];
  /** Target the findings were computed for; they are hidden once the view moves on. */
  reviewedTarget: string | null;
  reviewing: boolean;
  status: AssistantStatus | null;

  setCommitMessage: (message: string) => void;
  generate: () => Promise<void>;
  review: () => Promise<void>;
  loadStatus: () => Promise<void>;
  saveSettings: (settings: AssistantSettings) => Promise<void>;
}

const BUILT_IN = "built-in rules";
const notify = useUiStore.getState().notify;

export function targetKey(target: DiffTarget | null): string | null {
  if (!target) return null;
  return target.kind === "commit" ? `commit:${target.oid}` : target.kind;
}

export const useAssistStore = create<AssistState>((set, get) => ({
  commitMessage: "",
  draftSource: null,
  generating: false,
  findings: [],
  reviewedTarget: null,
  reviewing: false,
  status: null,

  setCommitMessage: (commitMessage) => set({ commitMessage, draftSource: commitMessage === "" ? null : get().draftSource }),

  generate: async () => {
    if (get().generating) return;
    set({ generating: true });
    try {
      const draft = await ipc.generateCommitMessage();
      set({ commitMessage: draft.message, draftSource: draft.model ?? BUILT_IN });
      if (draft.notice) notify("info", `Drafted with built-in rules: ${draft.notice}`);
    } catch (error) {
      notify("error", errorMessage(error));
    } finally {
      set({ generating: false });
    }
  },

  review: async () => {
    const target = useDiffStore.getState().target;
    if (!target || get().reviewing) return;
    set({ reviewing: true });
    try {
      const report = await ipc.reviewRisks(target);
      set({ findings: report.findings, reviewedTarget: targetKey(target) });
      const count = report.findings.length;
      const source = report.model ? `built-in rules and ${report.model}` : BUILT_IN;
      const outcome = count === 0 ? "No risks found" : `${count} ${count === 1 ? "risk" : "risks"} flagged`;
      notify("info", report.notice ? `${outcome} by built-in rules: ${report.notice}` : `${outcome} by ${source}`);
    } catch (error) {
      notify("error", errorMessage(error));
    } finally {
      set({ reviewing: false });
    }
  },

  loadStatus: async () => set({ status: await ipc.assistantStatus().catch(() => null) }),

  saveSettings: async (settings) => {
    try {
      const status = await ipc.saveAssistantSettings(settings);
      set({ status });
      notify("info", status.detail);
    } catch (error) {
      notify("error", errorMessage(error));
    }
  },
}));

/** Findings for the file on screen, or an empty list when the review belongs to another target. */
export function findingsFor(state: AssistState, target: DiffTarget | null, path: string | null): RiskFinding[] {
  if (state.reviewedTarget === null || state.reviewedTarget !== targetKey(target)) return [];
  return state.findings.filter((finding) => finding.path === path);
}
