import {
  Database,
  FolderOpen,
  HardDrive,
  Moon,
  Save,
  Sun,
  Monitor,
} from "lucide-react";
import { useEffect, useState } from "react";

import { api, errorMessage } from "@/lib/api";
import { formatNumber } from "@/lib/format";
import { useAppStore } from "@/stores/app";
import { useUiStore } from "@/stores/ui";
import type { Provider, UsageSummary } from "@/types";

const THEMES: {
  id: "system" | "light" | "dark";
  label: string;
  icon: typeof Monitor;
}[] = [
  { id: "system", label: "System", icon: Monitor },
  { id: "light", label: "Light", icon: Sun },
  { id: "dark", label: "Dark", icon: Moon },
];

const SETTING_KEYS = [
  "theme",
  "sendOnEnter",
  "contextMessageLimit",
  "maxOutputTokens",
  "taskMaxSteps",
  "ollamaKeepAlive",
  "appSystemPrompt",
] as const;

export function SettingsView() {
  const settings = useAppStore((state) => state.settings);
  const saveSettings = useAppStore((state) => state.saveSettings);
  const setTheme = useAppStore((state) => state.setTheme);
  const info = useAppStore((state) => state.info);
  const backup = useAppStore((state) => state.backup);
  const toast = useUiStore((state) => state.toast);

  const [draft, setDraft] = useState<Record<string, string>>(settings);
  const [busy, setBusy] = useState(false);
  const [usage, setUsage] = useState<UsageSummary[]>([]);
  const [providers, setProviders] = useState<Provider[]>([]);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  useEffect(() => {
    api
      .usageSummary()
      .then(setUsage)
      .catch(() => setUsage([]));
    api
      .listProviders()
      .then(setProviders)
      .catch(() => setProviders([]));
  }, []);

  const set = (key: string, value: string) =>
    setDraft((current) => ({ ...current, [key]: value }));

  const save = async () => {
    setBusy(true);
    const entries: Record<string, string> = {};
    for (const key of SETTING_KEYS) {
      const value = draft[key];
      if (value !== undefined) entries[key] = value;
    }
    const ok = await saveSettings(entries);
    setBusy(false);
    if (ok) toast("Settings saved.", "success");
  };

  const providerName = (providerId: string | null) =>
    providers.find((provider) => provider.id === providerId)?.name ?? providerId ?? "—";

  return (
    <div className="page">
      <div className="page-narrow">
        <div className="page-header">
          <div>
            <h1>Settings</h1>
            <p>Appearance, chat behavior, agent defaults, and local data.</p>
          </div>
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <h2>Appearance</h2>
          <div className="tabs" style={{ margin: "8px 0 4px" }}>
            {THEMES.map(({ id, label, icon: Icon }) => (
              <button
                key={id}
                className={draft.theme === id ? "active" : ""}
                onClick={() => {
                  set("theme", id);
                  void setTheme(id);
                }}
              >
                <Icon size={12} style={{ marginRight: 5, verticalAlign: -2 }} />
                {label}
              </button>
            ))}
          </div>
          <p className="faint small-text">
            System follows the macOS appearance automatically.
          </p>
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <h2>Chat</h2>
          <div className="field" style={{ marginTop: 10 }}>
            <label className="switch">
              <input
                type="checkbox"
                checked={draft.sendOnEnter !== "false"}
                onChange={(event) => set("sendOnEnter", event.target.checked ? "true" : "false")}
              />
              Enter sends the message (Shift+Enter for a new line)
            </label>
            <span className="hint">When off, use ⌘Enter to send.</span>
          </div>
          <div className="row wrap" style={{ gap: 14 }}>
            <div className="field" style={{ width: 180 }}>
              <label htmlFor="context-limit">Context messages</label>
              <input
                id="context-limit"
                type="number"
                min={2}
                max={200}
                value={draft.contextMessageLimit ?? "30"}
                onChange={(event) => set("contextMessageLimit", event.target.value)}
              />
              <span className="hint">History sent with each request (2–200).</span>
            </div>
            <div className="field" style={{ width: 180 }}>
              <label htmlFor="max-tokens">Max output tokens</label>
              <input
                id="max-tokens"
                type="number"
                min={64}
                max={32768}
                value={draft.maxOutputTokens ?? "4096"}
                onChange={(event) => set("maxOutputTokens", event.target.value)}
              />
              <span className="hint">64–32768.</span>
            </div>
            <div className="field" style={{ width: 180 }}>
              <label htmlFor="keep-alive">Ollama keep-alive</label>
              <input
                id="keep-alive"
                value={draft.ollamaKeepAlive ?? "10m"}
                onChange={(event) => set("ollamaKeepAlive", event.target.value)}
              />
              <span className="hint">e.g. 10m, 30m, 1h.</span>
            </div>
          </div>
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <h2>Agents</h2>
          <div className="field" style={{ width: 200, marginTop: 10 }}>
            <label htmlFor="task-steps">Maximum plan steps</label>
            <input
              id="task-steps"
              type="number"
              min={1}
              max={12}
              value={draft.taskMaxSteps ?? "8"}
              onChange={(event) => set("taskMaxSteps", event.target.value)}
            />
            <span className="hint">Plan size cap for every task (1–12).</span>
          </div>
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <h2>App system prompt</h2>
          <div className="field" style={{ marginTop: 10 }}>
            <textarea
              value={draft.appSystemPrompt ?? ""}
              placeholder="Optional guidance prepended to every conversation."
              onChange={(event) => set("appSystemPrompt", event.target.value)}
              style={{ minHeight: 100 }}
            />
            <span className="hint">Up to 20,000 characters. Leave blank to disable.</span>
          </div>
        </div>

        <div className="row" style={{ marginBottom: 18 }}>
          <button className="btn primary" onClick={() => void save()} disabled={busy}>
            <Save size={13} />
            {busy ? "Saving…" : "Save settings"}
          </button>
        </div>

        <div className="card card-pad" style={{ marginBottom: 14 }}>
          <h2>Usage</h2>
          {usage.length === 0 ? (
            <p className="faint small-text">No usage recorded yet.</p>
          ) : (
            <div className="list">
              {usage.map((row, index) => (
                <div className="list-item" key={`${row.providerId}-${row.model}-${index}`}>
                  <div className="grow">
                    <div className="title">{row.model ?? "Unknown model"}</div>
                    <div className="sub">{providerName(row.providerId)}</div>
                  </div>
                  <div className="right">
                    {formatNumber(row.requests)} req · {formatNumber(row.inputTokens)} in ·{" "}
                    {formatNumber(row.outputTokens)} out
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div className="card card-pad">
          <h2>Local data</h2>
          <p className="small-text">
            Everything is stored in a local SQLite database. API keys live in the macOS
            Keychain.
          </p>
          {info ? (
            <dl className="kv" style={{ margin: "10px 0" }}>
              <dt>Version</dt>
              <dd>{info.version}</dd>
              <dt>Data dir</dt>
              <dd>{info.dataDir}</dd>
              <dt>Database</dt>
              <dd>{info.dbPath}</dd>
              <dt>Logs</dt>
              <dd>{info.logDir}</dd>
              <dt>Artifacts</dt>
              <dd>{info.artifactsDir}</dd>
            </dl>
          ) : null}
          <div className="row wrap">
            <button className="btn small" onClick={() => void backup()}>
              <Database size={12} />
              Backup database
            </button>
            <button
              className="btn small"
              onClick={() => {
                void api.openDataDir().catch((error: unknown) => {
                  toast(errorMessage(error), "error");
                });
              }}
            >
              <FolderOpen size={12} />
              Open data folder
            </button>
            {info ? (
              <span className="faint small-text" style={{ display: "flex", gap: 5 }}>
                <HardDrive size={12} /> Project data never leaves this Mac.
              </span>
            ) : null}
          </div>
        </div>

        <div className="faint small-text" style={{ marginTop: 14 }}>
          Relay Routing is an independent local app. It is not affiliated with Anthropic,
          OpenRouter, or Google. Cloud provider usage requires your own API keys and
          billing.
        </div>
      </div>
    </div>
  );
}
