import { useEffect, useState } from "react";
import type { AssistantProvider, AssistantSettings } from "../../lib/types";
import { useAssistStore } from "../../store/useAssistStore";
import { useUiStore } from "../../store/useUiStore";

const PROVIDERS: { value: AssistantProvider | null; label: string; model: string; url: string }[] = [
  { value: null, label: "None (built-in rules only)", model: "", url: "" },
  { value: "anthropic", label: "Anthropic", model: "claude-opus-5-5", url: "https://api.anthropic.com" },
  { value: "openai", label: "OpenAI compatible", model: "model name", url: "https://api.openai.com/v1" },
  { value: "ollama", label: "Ollama (local)", model: "model name", url: "http://localhost:11434/v1" },
];

const FIELD = "h-8 w-full border border-line-strong bg-canvas px-2.5 font-mono text-[12px] text-ink outline-none focus:border-accent/70 placeholder:text-faint";

/** Provider settings for commit drafting and review. Everything here is optional. */
export function AssistantSettingsDialog() {
  const open = useUiStore((state) => state.settingsOpen);
  return open ? <Dialog /> : null;
}

function Dialog() {
  const close = () => useUiStore.getState().setSettingsOpen(false);
  const status = useAssistStore((state) => state.status);
  const saveSettings = useAssistStore((state) => state.saveSettings);
  const loadStatus = useAssistStore((state) => state.loadStatus);
  const [form, setForm] = useState<AssistantSettings>({ provider: null, base_url: null, model: null, api_key: null });

  useEffect(() => void loadStatus(), [loadStatus]);
  useEffect(() => {
    if (status) setForm({ provider: status.provider, base_url: status.base_url, model: status.model, api_key: null });
  }, [status]);

  const preset = PROVIDERS.find((provider) => provider.value === form.provider) ?? PROVIDERS[0];
  const field = (key: "base_url" | "model" | "api_key") => ({
    value: form[key] ?? "",
    onChange: (event: React.ChangeEvent<HTMLInputElement>) => setForm({ ...form, [key]: event.target.value || null }),
  });

  return (
    <div className="fixed inset-0 z-40 flex justify-center bg-black/35 pt-[16vh]" onPointerDown={close}>
      <form
        className="h-fit w-[460px] border border-line-strong bg-raised p-5 shadow-float"
        onPointerDown={(event) => event.stopPropagation()}
        onKeyDown={(event) => event.key === "Escape" && close()}
        onSubmit={(event) => {
          event.preventDefault();
          void saveSettings(form).then(close);
        }}
      >
        <h2 className="text-[14px] font-semibold">AI provider</h2>
        <p className="mt-1 text-[12px] leading-relaxed text-slate">
          Optional. Graft drafts commit messages and audits risks offline; a model adds its own findings. Only the structural summary of a diff is sent.
        </p>

        <label className="mt-4 block text-[11px] uppercase tracking-[0.06em] text-slate">
          Provider
          <select
            autoFocus
            value={form.provider ?? ""}
            onChange={(event) => setForm({ provider: (event.target.value || null) as AssistantProvider | null, base_url: null, model: null, api_key: null })}
            className={`${FIELD} mt-1 font-sans`}
          >
            {PROVIDERS.map((provider) => (
              <option key={provider.label} value={provider.value ?? ""}>
                {provider.label}
              </option>
            ))}
          </select>
        </label>
        {form.provider && (
          <>
            <label className="mt-3 block text-[11px] uppercase tracking-[0.06em] text-slate">
              Model
              <input {...field("model")} placeholder={preset?.model} spellCheck={false} className={`${FIELD} mt-1`} />
            </label>
            <label className="mt-3 block text-[11px] uppercase tracking-[0.06em] text-slate">
              Base URL
              <input {...field("base_url")} placeholder={preset?.url} spellCheck={false} className={`${FIELD} mt-1`} />
            </label>
            {form.provider !== "ollama" && (
              <label className="mt-3 block text-[11px] uppercase tracking-[0.06em] text-slate">
                API key
                <input {...field("api_key")} type="password" placeholder={status?.has_key ? "saved or set in the environment" : "stored locally, readable only by you"} className={`${FIELD} mt-1`} />
              </label>
            )}
          </>
        )}

        <div className="mt-5 flex items-center gap-3">
          <span className="min-w-0 flex-1 truncate text-[11.5px] text-faint">{status?.detail}</span>
          <button type="button" onClick={close} className="px-2 text-[12px] text-slate hover:text-ink">
            Cancel
          </button>
          <button type="submit" className="bg-accent px-3 text-[12px] font-medium leading-[24px] text-canvas hover:bg-accent/85">
            Save
          </button>
        </div>
      </form>
    </div>
  );
}
