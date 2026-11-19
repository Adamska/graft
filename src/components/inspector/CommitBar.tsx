import { useState } from "react";
import { errorMessage, ipc } from "../../lib/ipc";
import { commit } from "../../store/stagingActions";
import { useAssistStore } from "../../store/useAssistStore";
import { useRepoStore } from "../../store/useRepoStore";
import { useUiStore } from "../../store/useUiStore";
import { SparkIcon } from "../ui/icons";

export const COMMIT_BAR_INSET = 140;

/** Floating commit box shown over the diff while the working tree is selected. */
export function CommitBar() {
  const staged = useRepoStore((state) => state.status.staged.length);
  const changed = useRepoStore((state) => state.status.unstaged.length);
  const hasHead = useRepoStore((state) => state.repo?.head.oid != null);
  const message = useAssistStore((state) => state.commitMessage);
  const setMessage = useAssistStore((state) => state.setCommitMessage);
  const draftSource = useAssistStore((state) => state.draftSource);
  const generating = useAssistStore((state) => state.generating);
  const generate = useAssistStore((state) => state.generate);
  const [busy, setBusy] = useState(false);
  const [amend, setAmend] = useState(false);
  const [prefilled, setPrefilled] = useState("");
  const ready = !busy && (amend ? staged > 0 || message.trim() !== "" : staged > 0 && message.trim() !== "");

  const submit = async () => {
    if (!ready) return;
    setBusy(true);
    if (await commit(message, amend)) {
      setMessage("");
      setAmend(false);
    }
    setBusy(false);
  };

  /* Amending starts from the message of the commit being replaced; backing out removes it again. */
  const toggleAmend = async (next: boolean) => {
    setAmend(next);
    if (!next) return void (message === prefilled && setMessage(""));
    if (message.trim() !== "") return;
    try {
      const previous = await ipc.headMessage();
      setPrefilled(previous);
      setMessage(previous);
    } catch (error) {
      useUiStore.getState().notify("error", errorMessage(error));
    }
  };

  const placeholder = amend ? "Message for the amended commit" : staged > 0 ? "Commit message" : "Stage a file, a function, a hunk, or single lines to commit";
  return (
    <form
      className="absolute inset-x-4 bottom-3 z-20 rounded-panel border border-line-strong bg-raised shadow-float"
      onSubmit={(event) => {
        event.preventDefault();
        void submit();
      }}
    >
      <textarea
        data-commit-message
        value={message}
        onChange={(event) => setMessage(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) void submit();
        }}
        rows={Math.min(8, Math.max(2, message.split("\n").length))}
        spellCheck={false}
        placeholder={placeholder}
        className="block w-full resize-none bg-transparent px-3.5 pt-3 font-mono text-[12px] leading-[18px] text-ink outline-none placeholder:text-faint"
      />
      <div className="flex items-center gap-3 px-3.5 pb-2.5 pt-1 text-[11.5px]">
        <span className={`shrink-0 tabular-nums ${staged > 0 ? "text-add" : "text-faint"}`}>
          {staged} {staged === 1 ? "file" : "files"} staged
        </span>
        <label className={`flex shrink-0 items-center gap-1.5 ${hasHead ? "text-slate hover:text-ink" : "text-faint"}`} title="Replace the last commit instead of adding a new one">
          <input type="checkbox" checked={amend} disabled={!hasHead} onChange={(event) => void toggleAmend(event.target.checked)} className="size-3 accent-accent" />
          Amend
        </label>
        {draftSource && <span className="min-w-0 truncate text-faint">drafted by {draftSource}</span>}
        <button
          type="button"
          disabled={generating || staged + changed === 0}
          onClick={() => void generate()}
          title="Draft a Conventional Commit message from the structural diff"
          className="ml-auto flex h-6 shrink-0 items-center gap-1.5 rounded-control border border-line-strong px-2 text-ink hover:border-accent/60 hover:bg-accent/[0.06] disabled:text-faint"
        >
          <SparkIcon width={12} height={12} />
          {generating ? "Generating" : "Generate"}
        </button>
        <span className="shrink-0 font-mono text-[10.5px] text-faint">⌘↵</span>
        <button
          type="submit"
          disabled={!ready}
          className="h-6 shrink-0 rounded-control border border-transparent bg-accent px-3 text-[12px] font-medium text-canvas hover:bg-accent/85 disabled:border-line-strong disabled:bg-transparent disabled:text-faint"
        >
          {busy ? "Committing" : amend ? "Amend Commit" : "Commit"}
        </button>
      </div>
    </form>
  );
}
