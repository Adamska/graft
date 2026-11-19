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
      className={`fixed left-1/2 top-12 z-50 max-w-[560px] -translate-x-1/2 select-text border bg-raised px-3.5 py-2 text-left text-[12.5px] shadow-float ${
        notice.tone === "error" ? "border-del/50 text-del" : "border-line-strong text-ink"
      }`}
    >
      {notice.text}
    </button>
  );
}
