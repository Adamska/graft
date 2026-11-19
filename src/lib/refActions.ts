import { useRepoStore } from "../store/useRepoStore";
import { type DialogValues, useUiStore } from "../store/useUiStore";
import { perform } from "./actions";
import { ipc } from "./ipc";
import type { Stash } from "./types";

const ask = () => useUiStore.getState().ask;
const text = (values: DialogValues, key: string) => String(values[key] ?? "").trim();

/** Branch, tag, and stash commands that need a name or a confirmation first. */
export const refActions = {
  /** `start` is any revision; the branch starts at `HEAD` when it is omitted. */
  createBranch: (start: string | null = null, from?: string) =>
    ask()({
      title: from ? `New branch at ${from}` : "New branch",
      fields: [
        { key: "name", label: "Branch name", kind: "text", required: true, placeholder: "feature/short-description" },
        { key: "checkout", label: "Check out the new branch", kind: "checkbox", initial: true },
      ],
      confirm: "Create Branch",
      onConfirm: (values) => {
        const name = text(values, "name");
        return perform("Creating branch", () => ipc.createBranch(name, start, values.checkout === true), `Created ${name}`);
      },
    }),

  renameBranch: (name: string) =>
    ask()({
      title: `Rename ${name}`,
      fields: [{ key: "name", label: "New name", kind: "text", required: true, initial: name }],
      confirm: "Rename",
      onConfirm: (values) => perform("Renaming", () => ipc.renameBranch(name, text(values, "name")), `Renamed ${name} to ${text(values, "name")}`),
    }),

  deleteBranch: (name: string) =>
    ask()({
      title: `Delete branch ${name}?`,
      message: "Only the branch pointer is removed. Git refuses if its commits are not merged anywhere, unless you force it.",
      fields: [{ key: "force", label: "Delete even if not merged", kind: "checkbox", initial: false }],
      confirm: "Delete",
      danger: true,
      onConfirm: (values) => perform("Deleting", () => ipc.deleteBranch(name, values.force === true, false), `Deleted ${name}`),
    }),

  deleteRemoteBranch: (name: string) =>
    ask()({
      title: `Delete ${name} from its remote?`,
      message: "The branch is removed for everyone who uses the remote. Local branches are not affected.",
      confirm: "Delete Remote Branch",
      danger: true,
      onConfirm: () => perform("Deleting", () => ipc.deleteBranch(name, false, true), `Deleted ${name}`),
    }),

  createTag: (target: string, at: string) =>
    ask()({
      title: `New tag at ${at}`,
      fields: [
        { key: "name", label: "Tag name", kind: "text", required: true, placeholder: "v1.0.0" },
        { key: "message", label: "Message (makes the tag annotated)", kind: "text", placeholder: "Optional" },
      ],
      confirm: "Create Tag",
      onConfirm: (values) => perform("Tagging", () => ipc.createTag(text(values, "name"), target, text(values, "message") || null), `Tagged ${at} as ${text(values, "name")}`),
    }),

  deleteTag: (name: string) =>
    ask()({
      title: `Delete tag ${name}?`,
      message: "The tag is removed locally. A copy that was already pushed stays on the remote.",
      confirm: "Delete",
      danger: true,
      onConfirm: () => perform("Deleting", () => ipc.deleteTag(name), `Deleted tag ${name}`),
    }),

  pushTag: (name: string) => perform("Pushing", () => ipc.pushTag(name)),

  stash: () =>
    ask()({
      title: "Stash changes",
      message: "Shelves everything that is not committed and returns the working tree to the last commit.",
      fields: [
        { key: "message", label: "Description", kind: "text", placeholder: "Optional" },
        { key: "untracked", label: "Include untracked files", kind: "checkbox", initial: true },
      ],
      confirm: "Stash",
      onConfirm: (values) => perform("Stashing", () => ipc.stashSave(text(values, "message"), values.untracked === true), "Changes stashed"),
    }),

  /** Restores a stash; popping also removes it from the list once it applied cleanly. */
  applyStash: async (stash: Stash, pop: boolean) => {
    const applied = await perform(pop ? "Popping stash" : "Applying stash", () => ipc.stashApply(stash.index, pop), pop ? "Stash popped" : "Stash applied");
    if (applied) useRepoStore.getState().selectWorking();
  },

  dropStash: (stash: Stash) =>
    ask()({
      title: "Drop this stash?",
      message: `"${stash.message}" will be deleted without being applied.`,
      confirm: "Drop",
      danger: true,
      onConfirm: async () => {
        useRepoStore.getState().selectWorking();
        await perform("Dropping stash", () => ipc.stashDrop(stash.index), "Stash dropped");
      },
    }),
};
