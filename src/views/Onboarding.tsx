import {
  ArrowRight,
  CheckCircle2,
  CircleAlert,
  Cpu,
  KeyRound,
  Loader2,
  Lock,
  Server,
} from "lucide-react";
import { useEffect, useState } from "react";

import { useAppStore } from "@/stores/app";
import { useProvidersStore } from "@/stores/providers";
import { useUiStore } from "@/stores/ui";

export function Onboarding() {
  const completeOnboarding = useAppStore((state) => state.completeOnboarding);
  const info = useAppStore((state) => state.info);
  const providers = useProvidersStore((state) => state.providers);
  const accounts = useProvidersStore((state) => state.accounts);
  const healthFor = useProvidersStore((state) => state.healthFor);
  const navigate = useUiStore((state) => state.navigate);
  const [ollamaStatus, setOllamaStatus] = useState<
    "checking" | "online" | "offline" | "missing"
  >("checking");
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);

  const ollama = providers.find((provider) => provider.kind === "ollama");
  const ollamaAccount = accounts.find(
    (account) => account.providerId === ollama?.id,
  );
  const cloudProviderIds = new Set(
    providers.filter((provider) => provider.kind !== "ollama").map((provider) => provider.id),
  );
  const hasCloudKey = accounts.some(
    (account) => account.hasSecret && cloudProviderIds.has(account.providerId),
  );

  useEffect(() => {
    let live = true;
    if (!ollama) {
      setOllamaStatus("missing");
      return;
    }
    void healthFor(ollama.id, ollamaAccount?.id).then((result) => {
      if (!live) return;
      if (!result) {
        setOllamaStatus("offline");
        setMessage("Health check failed.");
        return;
      }
      setOllamaStatus(result.ok ? "online" : "offline");
      setMessage(
        result.ok
          ? `Ollama responded in ${result.latencyMs} ms${result.version ? ` (v${result.version})` : ""}.`
          : result.message,
      );
    });
    return () => {
      live = false;
    };
  }, [ollama, ollamaAccount?.id, healthFor]);

  const finish = async (destination: "chat" | "providers") => {
    setBusy(true);
    await completeOnboarding();
    if (destination === "providers") navigate({ name: "providers" });
    setBusy(false);
  };

  return (
    <div className="onboarding">
      <div className="onboarding-card">
        <div className="onboarding-hero">
          <div className="onboarding-mark">CM</div>
          <h1>Welcome to Claude Multi</h1>
          <p>
            A local-first workspace for chatting with local and cloud models, organizing
            projects, and running durable research tasks.
          </p>
        </div>

        <div className="card card-pad" style={{ marginBottom: 12 }}>
          <div className="check-list" style={{ margin: 0 }}>
            <div className="item">
              <Lock size={15} />
              <span>
                <strong>Private by default.</strong> Conversations, attachments, artifacts,
                and usage live in a local SQLite database on this Mac. API keys are stored in
                the macOS Keychain.
              </span>
            </div>
            <div className="item">
              <Server size={15} />
              <span>
                <strong>Local models via Ollama.</strong>{" "}
                {ollamaStatus === "checking" ? (
                  <span className="row" style={{ display: "inline-flex", gap: 6 }}>
                    <Loader2 size={12} className="spin" /> Checking…
                  </span>
                ) : ollamaStatus === "online" ? (
                  <span style={{ color: "var(--success)" }}>
                    <CheckCircle2 size={12} style={{ verticalAlign: -2 }} /> Online.{" "}
                    {message}
                  </span>
                ) : (
                  <span style={{ color: "var(--warning)" }}>
                    <CircleAlert size={12} style={{ verticalAlign: -2 }} />{" "}
                    {ollamaStatus === "missing"
                      ? "Provider row missing — reinstall the app data."
                      : `Not reachable. ${message || "Start the Ollama app, then check Providers."}`}
                  </span>
                )}
              </span>
            </div>
            <div className="item">
              <KeyRound size={15} />
              <span>
                <strong>Cloud models via API key.</strong>{" "}
                {hasCloudKey
                  ? "A key is saved for at least one cloud account."
                  : "Add your own Anthropic, OpenRouter, or Google Gemini API key in Providers. Subscription logins are not supported."}
              </span>
            </div>
            <div className="item">
              <Cpu size={15} />
              <span>
                <strong>Durable tasks.</strong> Agents plan, execute sequential steps, save
                checkpoints, and write Markdown reports you can open locally.
              </span>
            </div>
          </div>
        </div>

        <div className="row" style={{ justifyContent: "flex-end" }}>
          <button className="btn" onClick={() => void finish("providers")} disabled={busy}>
            Configure providers
          </button>
          <button
            className="btn primary"
            onClick={() => void finish("chat")}
            disabled={busy}
          >
            Start chatting
            <ArrowRight size={14} />
          </button>
        </div>
        <div className="faint small-text" style={{ textAlign: "center", marginTop: 12 }}>
          {info ? `Version ${info.version} · ${info.identifier}` : ""}
        </div>
      </div>
    </div>
  );
}
