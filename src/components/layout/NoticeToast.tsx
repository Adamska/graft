import { useUiStore } from "../../store/useUiStore";

export function NoticeToast() {
  const notice = useUiStore((state) => state.notice);
  const dismiss = useUiStore((state) => state.dismissNotice);
  if (!notice) return null;

  return (
    <button
      type="button"
      role="status"
      onClick={dismiss}
      className={`fixed bottom-9 left-4 z-50 max-w-[520px] select-text rounded-panel border bg-raised px-3.5 py-2 text-left text-[12.5px] shadow-float ${
        notice.tone === "error" ? "border-del/50 text-del" : "border-line-strong text-ink"
      }`}
    >
      {notice.text}
    </button>
  );
}
