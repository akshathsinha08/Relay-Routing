import {
  Check,
  CircleAlert,
  Cpu,
  KeyRound,
  Loader2,
  Plus,
  RefreshCw,
  ShieldCheck,
  Trash2,
  X,
} from "lucide-react";
import { useState } from "react";

import { relativeTime } from "@/lib/format";
import { useProvidersStore } from "@/stores/providers";
import { useUiStore } from "@/stores/ui";
import type { Account, Provider } from "@/types";

function AccountRow({
  account,
  provider,
}: {
  account: Account;
  provider: Provider;
}) {
  const saveSecret = useProvidersStore((state) => state.saveSecret);
  const testAccount = useProvidersStore((state) => state.testAccount);
  const renameAccount = useProvidersStore((state) => state.renameAccount);
  const deleteAccount = useProvidersStore((state) => state.deleteAccount);
  const cachedModels = useProvidersStore((state) => state.cachedModels);
  const ensureModels = useProvidersStore((state) => state.ensureModels);
  const refreshAccounts = useProvidersStore((state) => state.refreshAccounts);
  const toast = useUiStore((state) => state.toast);

  const [editingSecret, setEditingSecret] = useState(false);
  const [secret, setSecret] = useState("");
  const [testing, setTesting] = useState(false);
  const [renaming, setRenaming] = useState(false);
  const [label, setLabel] = useState(account.label);
  const models = cachedModels(provider.id, account.id);

  const save = async () => {
    if (!secret.trim()) return;
    const ok = await saveSecret(account.id, secret.trim());
    if (ok) {
      setEditingSecret(false);
      setSecret("");
      toast("Credential saved to macOS Keychain.", "success");
      await ensureModels(provider.id, account.id);
    }
  };

  const runTest = async () => {
    setTesting(true);
    const result = await testAccount(account.id);
    setTesting(false);
    if (result) {
      toast(
        result.ok
          ? `Connected in ${result.latencyMs} ms · ${result.modelCount} model(s)`
          : result.message,
        result.ok ? "success" : "error",
      );
    }
  };

  return (
    <div className="list-item" style={{ alignItems: "flex-start" }}>
      <div className="grow">
        <div className="row" style={{ gap: 6 }}>
          {renaming ? (
            <>
              <input
                value={label}
                autoFocus
                style={{ maxWidth: 220 }}
                onChange={(event) => setLabel(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter") {
                    void renameAccount(account.id, label.trim() || account.label).then(() => {
                      setRenaming(false);
                    });
                  }
                  if (event.key === "Escape") setRenaming(false);
                }}
              />
              <button
                className="icon-btn"
                onClick={() => {
                  void renameAccount(account.id, label.trim() || account.label).then(() => {
                    setRenaming(false);
                  });
                }}
              >
                <Check size={13} />
              </button>
              <button className="icon-btn" onClick={() => setRenaming(false)}>
                <X size={13} />
              </button>
            </>
          ) : (
            <>
              <span className="title">{account.label}</span>
              <button
                className="btn small ghost"
                onClick={() => {
                  setLabel(account.label);
                  setRenaming(true);
                }}
              >
                Rename
              </button>
            </>
          )}
          <span className={`badge ${account.status === "error" ? "danger" : "success"}`}>
            {account.status === "error" ? "Needs attention" : "Ready"}
          </span>
        </div>
        <div className="sub" style={{ marginTop: 2 }}>
          {account.authKind === "none"
            ? "No credential required"
            : account.hasSecret
              ? "API key stored in Keychain"
              : "No API key saved"}{" "}
          · updated {relativeTime(account.updatedAt)}
        </div>
        {account.statusMessage ? (
          <div className="small-text" style={{ color: "var(--danger)", marginTop: 2 }}>
            {account.statusMessage}
          </div>
        ) : null}

        {editingSecret ? (
          <div className="secret-row" style={{ marginTop: 8 }}>
            <input
              type="password"
              autoFocus
              value={secret}
              placeholder={account.hasSecret ? "Replace API key" : "Paste API key"}
              onChange={(event) => setSecret(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter") void save();
                if (event.key === "Escape") {
                  setEditingSecret(false);
                  setSecret("");
                }
              }}
            />
            <button className="btn primary small" onClick={() => void save()}>
              Save
            </button>
            <button
              className="btn ghost small"
              onClick={() => {
                setEditingSecret(false);
                setSecret("");
              }}
            >
              Cancel
            </button>
          </div>
        ) : null}

        <div className="row wrap" style={{ marginTop: 8 }}>
          {account.authKind !== "none" ? (
            <button
              className="btn small"
              onClick={() => setEditingSecret((value) => !value)}
            >
              <KeyRound size={12} />
              {account.hasSecret ? "Replace key" : "Add key"}
            </button>
          ) : null}
          <button className="btn small" onClick={() => void runTest()} disabled={testing}>
            {testing ? <Loader2 size={12} className="spin" /> : <RefreshCw size={12} />}
            Test connection
          </button>
          <button
            className="btn small ghost"
            onClick={() => {
              void ensureModels(provider.id, account.id);
              void refreshAccounts();
            }}
          >
            {models.length ? `${models.length} model(s)` : "Load models"}
          </button>
          <button
            className="btn small danger"
            onClick={() => {
              if (confirm(`Delete account "${account.label}"? Its stored key is removed.`)) {
                void deleteAccount(account.id);
              }
            }}
          >
            <Trash2 size={12} />
            Remove
          </button>
        </div>
      </div>
    </div>
  );
}

export function ProvidersView() {
  const providers = useProvidersStore((state) => state.providers);
  const accounts = useProvidersStore((state) => state.accounts);
  const updateProvider = useProvidersStore((state) => state.updateProvider);
  const createAccount = useProvidersStore((state) => state.createAccount);
  const healthFor = useProvidersStore((state) => state.healthFor);
  const toast = useUiStore((state) => state.toast);

  const [baseUrls, setBaseUrls] = useState<Record<string, string>>({});
  const [health, setHealth] = useState<Record<string, string>>({});
  const [checking, setChecking] = useState<string | null>(null);
  const [addingFor, setAddingFor] = useState<string | null>(null);
  const [newLabel, setNewLabel] = useState("");
  const [newSecret, setNewSecret] = useState("");
  const [busy, setBusy] = useState(false);

  const check = async (provider: Provider) => {
    setChecking(provider.id);
    const account = accounts.find((item) => item.providerId === provider.id);
    const result = await healthFor(provider.id, account?.id);
    setChecking(null);
    setHealth((current) => ({
      ...current,
      [provider.id]: result
        ? result.ok
          ? `Online · ${result.latencyMs} ms${result.version ? ` · v${result.version}` : ""}`
          : result.message
        : "Check failed",
    }));
  };

  const addAccount = async (provider: Provider) => {
    if (!newLabel.trim()) return;
    const authKind = provider.kind === "ollama" ? "none" : "api_key";
    if (authKind === "api_key" && !newSecret.trim()) {
      toast("Paste an API key for this account.", "error");
      return;
    }
    setBusy(true);
    const created = await createAccount(
      provider.id,
      newLabel.trim(),
      authKind,
      newSecret.trim() || undefined,
    );
    setBusy(false);
    if (created) {
      setAddingFor(null);
      setNewLabel("");
      setNewSecret("");
      toast("Account added.", "success");
    }
  };

  return (
    <div className="page">
      <div className="page-narrow">
        <div className="page-header">
          <div>
            <h1>Providers</h1>
            <p>
              Local Ollama and Anthropic API accounts. Credentials are stored in the
              macOS Keychain — never in the database.
            </p>
          </div>
        </div>

        {providers.map((provider) => {
          const providerAccounts = accounts.filter(
            (account) => account.providerId === provider.id,
          );
          const baseUrl = baseUrls[provider.id] ?? provider.baseUrl;
          return (
            <div className="card card-pad" key={provider.id} style={{ marginBottom: 14 }}>
              <div className="row">
                <div className="grow">
                  <div className="row" style={{ gap: 8 }}>
                    <h2 style={{ margin: 0 }}>{provider.name}</h2>
                    <span className="badge">{provider.kind}</span>
                    <span className={`badge ${provider.enabled ? "success" : "warning"}`}>
                      {provider.enabled ? "Enabled" : "Disabled"}
                    </span>
                  </div>
                  <p className="small-text">
                    {provider.kind === "ollama"
                      ? "Runs models locally through the Ollama daemon."
                      : "Uses an Anthropic API key. Claude subscription logins are not supported."}
                  </p>
                </div>
                <label className="switch">
                  <input
                    type="checkbox"
                    checked={provider.enabled}
                    onChange={(event) => {
                      void updateProvider(provider.id, { enabled: event.target.checked });
                    }}
                  />
                  Enabled
                </label>
              </div>

              <div className="row wrap" style={{ alignItems: "flex-end", gap: 8, marginTop: 8 }}>
                <div className="field grow" style={{ marginBottom: 0 }}>
                  <label htmlFor={`base-${provider.id}`}>Base URL</label>
                  <input
                    id={`base-${provider.id}`}
                    value={baseUrl}
                    onChange={(event) =>
                      setBaseUrls((current) => ({
                        ...current,
                        [provider.id]: event.target.value,
                      }))
                    }
                  />
                </div>
                <button
                  className="btn small"
                  onClick={() => {
                    void updateProvider(provider.id, { baseUrl: baseUrl.trim() }).then(() => {
                      toast("Base URL updated.", "success");
                    });
                  }}
                >
                  Save URL
                </button>
                <button
                  className="btn small"
                  onClick={() => void check(provider)}
                  disabled={checking === provider.id}
                >
                  {checking === provider.id ? (
                    <Loader2 size={12} className="spin" />
                  ) : (
                    <ShieldCheck size={12} />
                  )}
                  Check health
                </button>
              </div>
              {health[provider.id] ? (
                <div
                  className="small-text"
                  style={{
                    marginTop: 6,
                    color: health[provider.id]?.startsWith("Online")
                      ? "var(--success)"
                      : "var(--danger)",
                  }}
                >
                  {health[provider.id]}
                </div>
              ) : null}

              <div className="divider" />
              <div className="row between" style={{ marginBottom: 6 }}>
                <h3 style={{ margin: 0 }}>Accounts</h3>
                <button
                  className="btn small"
                  onClick={() => {
                    setAddingFor(addingFor === provider.id ? null : provider.id);
                    setNewLabel("");
                    setNewSecret("");
                  }}
                >
                  <Plus size={12} />
                  Add account
                </button>
              </div>

              {addingFor === provider.id ? (
                <div className="card card-pad" style={{ marginBottom: 10 }}>
                  <div className="field">
                    <label htmlFor={`label-${provider.id}`}>Label</label>
                    <input
                      id={`label-${provider.id}`}
                      autoFocus
                      value={newLabel}
                      placeholder="e.g. Personal key"
                      onChange={(event) => setNewLabel(event.target.value)}
                    />
                  </div>
                  {provider.kind !== "ollama" ? (
                    <div className="field">
                      <label htmlFor={`secret-${provider.id}`}>API key</label>
                      <input
                        id={`secret-${provider.id}`}
                        type="password"
                        value={newSecret}
                        placeholder="sk-ant-…"
                        onChange={(event) => setNewSecret(event.target.value)}
                      />
                      <span className="hint">
                        Stored in the macOS Keychain under this account only.
                      </span>
                    </div>
                  ) : null}
                  <div className="row">
                    <button
                      className="btn primary small"
                      onClick={() => void addAccount(provider)}
                      disabled={busy || !newLabel.trim()}
                    >
                      Add account
                    </button>
                    <button className="btn ghost small" onClick={() => setAddingFor(null)}>
                      Cancel
                    </button>
                  </div>
                </div>
              ) : null}

              {providerAccounts.length === 0 ? (
                <p className="faint small-text" style={{ margin: 0 }}>
                  No accounts yet.
                </p>
              ) : (
                <div className="list">
                  {providerAccounts.map((account) => (
                    <AccountRow key={account.id} account={account} provider={provider} />
                  ))}
                </div>
              )}

              <div className="faint small-text" style={{ marginTop: 10, display: "flex", gap: 6 }}>
                {provider.kind === "ollama" ? <Cpu size={12} /> : <CircleAlert size={12} />}
                {provider.kind === "ollama"
                  ? "Model list comes from the local Ollama daemon. Pull models with `ollama pull`."
                  : "Anthropic requests are sent directly to api.anthropic.com with your key."}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
