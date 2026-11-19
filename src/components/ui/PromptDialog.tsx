import { useState } from "react";
import { type DialogSpec, type DialogValues, useUiStore } from "../../store/useUiStore";

/** Confirmations and small forms requested through the UI store. */
export function PromptDialog() {
  const dialog = useUiStore((state) => state.dialog);
  return dialog ? <Dialog key={dialog.title} dialog={dialog} /> : null;
}

function Dialog({ dialog }: { dialog: DialogSpec }) {
  const close = () => useUiStore.getState().ask(null);
  const fields = dialog.fields ?? [];
  const [values, setValues] = useState<DialogValues>(() =>
    Object.fromEntries(fields.map((field) => [field.key, field.initial ?? (field.kind === "checkbox" ? false : "")])),
  );
  const incomplete = fields.some((field) => field.required && String(values[field.key] ?? "").trim() === "");
  const hasText = fields.some((field) => field.kind === "text");

  const submit = () => {
    if (incomplete) return;
    close();
    void dialog.onConfirm(values);
  };

  return (
    <div className="fixed inset-0 z-50 flex justify-center bg-black/35 pt-[18vh]" onPointerDown={close}>
      <form
        role="dialog"
        aria-label={dialog.title}
        className="h-fit w-[440px] rounded-panel border border-line-strong bg-raised p-5 shadow-float"
        onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          if (event.key === "Escape") close();
          event.stopPropagation();
        }}
        onSubmit={(event) => {
          event.preventDefault();
          submit();
        }}
      >
        <h2 className="select-text break-words text-[14px] font-semibold leading-snug">{dialog.title}</h2>
        {dialog.message && <p className="mt-1.5 select-text text-[12.5px] leading-relaxed text-slate">{dialog.message}</p>}

        {fields.map((field, index) =>
          field.kind === "checkbox" ? (
            <label key={field.key} className="mt-3.5 flex items-center gap-2 text-[12.5px] text-soft">
              <input
                type="checkbox"
                checked={values[field.key] === true}
                onChange={(event) => setValues({ ...values, [field.key]: event.target.checked })}
                className="size-3.5 accent-accent"
              />
              {field.label}
            </label>
          ) : (
            <label key={field.key} className="mt-3.5 block text-[11.5px] text-slate">
              {field.label}
              <input
                autoFocus={index === 0}
                value={String(values[field.key] ?? "")}
                onChange={(event) => setValues({ ...values, [field.key]: event.target.value })}
                onFocus={(event) => event.target.select()}
                placeholder={field.placeholder}
                spellCheck={false}
                className="mt-1 h-8 w-full rounded-control border border-line-strong bg-canvas px-2.5 font-mono text-[12px] text-ink outline-none placeholder:text-faint focus:border-accent/70"
              />
            </label>
          ),
        )}

        <div className="mt-5 flex justify-end gap-2">
          <button type="button" onClick={close} className="h-7 rounded-control px-3 text-[12.5px] text-slate hover:bg-hover hover:text-ink">
            Cancel
          </button>
          <button
            type="submit"
            autoFocus={!hasText}
            disabled={incomplete}
            className={`h-7 rounded-control px-3.5 text-[12.5px] font-medium text-canvas disabled:opacity-40 ${
              dialog.danger ? "bg-del hover:bg-del/85" : "bg-accent hover:bg-accent/85"
            }`}
          >
            {dialog.confirm}
          </button>
        </div>
      </form>
    </div>
  );
}
