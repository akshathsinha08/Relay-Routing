import { Check, ChevronDown, Cpu, TriangleAlert } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { formatBytes, modelLabel, supportsTools } from "@/lib/format";
import { useChatStore } from "@/stores/chat";
import { useProvidersStore } from "@/stores/providers";

export function ModelPicker() {
  const conversation = useChatStore((state) =>
    state.conversations.find((item) => item.id === state.activeId),
  );
  const setTarget = useChatStore((state) => state.setTarget);
  const providers = useProvidersStore((state) => state.providers);
  const accounts = useProvidersStore((state) => state.accounts);
  const ensureModels = useProvidersStore((state) => state.ensureModels);
  const cachedModels = useProvidersStore((state) => state.cachedModels);
  const [open, setOpen] = useState(false);
  const [expanded, setExpanded] = useState<string | null>(
    conversation ? `${conversation.providerId}:${conversation.accountId}` : null,
  );
  const [loadingKey, setLoadingKey] = useState<string | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (event: MouseEvent) => {
      if (ref.current && !ref.current.contains(event.target as Node)) setOpen(false);
    };
    window.addEventListener("mousedown", handler);
    return () => window.removeEventListener("mousedown", handler);
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const providerId = conversation?.providerId;
    const accountId = conversation?.accountId ?? undefined;
    if (!providerId) return;
    if (cachedModels(providerId, accountId).length > 0) return;
    setLoadingKey(`${providerId}:${accountId ?? "default"}`);
    void ensureModels(providerId, accountId).finally(() => setLoadingKey(null));
  }, [open, conversation?.providerId, conversation?.accountId, cachedModels, ensureModels]);

  if (!conversation) return null;
  const enabledProviders = providers.filter((provider) => provider.enabled);
  const currentLabel = conversation.model ?? "Select model";
  const noTarget = !conversation.providerId || !conversation.model;

  const expand = async (providerId: string, accountId: string) => {
    const key = `${providerId}:${accountId}`;
    setExpanded(key);
    if (cachedModels(providerId, accountId).length === 0) {
      setLoadingKey(`${key}:loading`);
      await ensureModels(providerId, accountId);
      setLoadingKey(null);
    }
  };

  return (
    <div className="model-picker" ref={ref}>
      <button
        className="model-trigger"
        onClick={() => setOpen((value) => !value)}
        title="Change provider and model"
      >
        {noTarget ? <TriangleAlert size={13} /> : <Cpu size={13} />}
        <span className="model-name">{currentLabel}</span>
        <ChevronDown size={12} />
      </button>

      {open ? (
        <div className="popover">
          <div className="popover-section">Provider account</div>
          {enabledProviders.map((provider) => {
            const providerAccounts = accounts.filter(
              (account) => account.providerId === provider.id,
            );
            if (providerAccounts.length === 0) {
              return (
                <div key={provider.id} className="option" style={{ opacity: 0.6 }}>
                  <div className="option-main">
                    <div className="name">{provider.name}</div>
                    <div className="meta">No account configured</div>
                  </div>
                </div>
              );
            }
            return providerAccounts.map((account) => {
              const key = `${provider.id}:${account.id}`;
              const isExpanded = expanded === key;
              const isCurrent =
                conversation.providerId === provider.id &&
                conversation.accountId === account.id;
              return (
                <div key={key}>
                  <button className="option" onClick={() => void expand(provider.id, account.id)}>
                    <div className="option-main grow">
                      <div className="name">
                        {provider.name} · {account.label}
                      </div>
                      <div className="meta">
                        {account.status === "error"
                          ? account.statusMessage ?? "Connection error"
                          : account.hasSecret
                            ? "Credential stored in Keychain"
                            : account.authKind === "none"
                              ? "Local · no credential needed"
                              : "No credential saved"}
                      </div>
                    </div>
                    {isCurrent ? <Check size={13} className="check" /> : null}
                  </button>
                  {isExpanded ? (
                    <div style={{ paddingLeft: 10 }}>
                      {loadingKey?.startsWith(key) ? (
                        <div className="faint small-text" style={{ padding: "6px 10px" }}>
                          Loading models…
                        </div>
                      ) : cachedModels(provider.id, account.id).length === 0 ? (
                        <div className="faint small-text" style={{ padding: "6px 10px" }}>
                          No models reported. {provider.kind === "ollama" ? "Start Ollama or pull a model." : "Check the credential."}
                        </div>
                      ) : (
                        cachedModels(provider.id, account.id).map((model) => {
                          const selected =
                            isCurrent && conversation.model === model.id;
                          return (
                            <button
                              key={model.id}
                              className={`option ${selected ? "selected" : ""}`}
                              onClick={() => {
                                void setTarget(provider.id, account.id, model.id);
                                setOpen(false);
                              }}
                            >
                              <div className="option-main grow">
                                <div className="name">{modelLabel(model)}</div>
                                <div className="meta">
                                  {[
                                    model.sizeBytes ? formatBytes(model.sizeBytes) : null,
                                    model.family,
                                    supportsTools(model) ? "tools" : null,
                                    model.capabilities.includes("vision") ? "vision" : null,
                                  ]
                                    .filter(Boolean)
                                    .join(" · ") || "—"}
                                </div>
                              </div>
                              {selected ? <Check size={13} className="check" /> : null}
                            </button>
                          );
                        })
                      )}
                    </div>
                  ) : null}
                </div>
              );
            });
          })}
          <div className="popover-divider" />
          <div className="faint small-text" style={{ padding: "2px 10px 6px" }}>
            Chat runs on your selected provider. Cloud providers use your own API keys —
            subscription logins are not supported.
          </div>
        </div>
      ) : null}
    </div>
  );
}
