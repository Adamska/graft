import { useState } from "react";
import { useAssistStore } from "../../store/useAssistStore";
import { useDiffStore } from "../../store/useDiffStore";
import { useRepoStore } from "../../store/useRepoStore";
import { SparkIcon } from "../ui/icons";

export const COMMIT_BAR_INSET = 132;

/** Floating commit box shown over the diff while the working tree is selected. */
export function CommitBar() {
  const staged = useRepoStore((state) => state.status.staged.length);
  const commit = useDiffStore((state) => state.commit);
  const changed = useRepoStore((state) => state.status.unstaged.length);
  const message = useAssistStore((state) => state.commitMessage);
  const setMessage = useAssistStore((state) => state.setCommitMessage);
  const draftSource = useAssistStore((state) => state.draftSource);
  const generating = useAssistStore((state) => state.generating);
  const generate = useAssistStore((state) => state.generate);
  const [busy, setBusy] = useState(false);
  const ready = staged > 0 && message.trim() !== "" && !busy;

  const submit = async () => {
    if (!ready) return;
    setBusy(true);
    if (await commit(message)) setMessage("");
    setBusy(false);
  };

  return (
    <form
      className="absolute inset-x-4 bottom-3 z-20 border border-line-strong bg-raised shadow-[0_10px_40px_rgba(0,0,0,0.55)]"
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
        placeholder={staged > 0 ? "Commit message" : "Stage a file or a single function to commit"}
        className="block w-full resize-none bg-transparent px-3 pt-2.5 font-mono text-[12px] leading-[18px] text-ink outline-none placeholder:text-faint"
      />
      <div className="flex items-center gap-3 px-3 pb-2 pt-1 text-[11.5px]">
        <span className={`tabular-nums ${staged > 0 ? "text-add" : "text-faint"}`}>
          {staged} {staged === 1 ? "file" : "files"} staged
        </span>
        {draftSource && <span className="truncate text-faint">drafted by {draftSource}</span>}
        <button
          type="button"
          disabled={generating || staged + changed === 0}
          onClick={() => void generate()}
          title="Draft a Conventional Commit message from the structural diff"
          className="ml-auto flex items-center gap-1.5 border border-line-strong px-2 leading-[22px] text-ink hover:border-moved/60 hover:bg-moved/[0.06] disabled:text-faint"
        >
          <SparkIcon width={12} height={12} />
          {generating ? "Generating" : "Generate"}
        </button>
        <span className="font-mono text-[10.5px] text-faint">⌘↵</span>
        <button
          type="submit"
          disabled={!ready}
          className="border border-transparent bg-ink px-3 text-[12px] font-medium leading-[22px] text-obsidian hover:bg-white disabled:border-line-strong disabled:bg-transparent disabled:text-faint"
        >
          {busy ? "Committing" : "Commit"}
        </button>
      </div>
    </form>
  );
}
