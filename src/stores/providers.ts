import { create } from "zustand";

import { api, errorMessage } from "@/lib/api";
import type {
  Account,
  AccountTest,
  ModelInfo,
  Provider,
  ProviderHealth,
} from "@/types";

import { useUiStore } from "./ui";

export interface ModelSelection {
  providerId: string;
  accountId: string;
  model: string;
}

interface ProvidersState {
  providers: Provider[];
  accounts: Account[];
  models: Record<string, ModelInfo[]>;
  health: Record<string, ProviderHealth>;
  loading: boolean;
  load: () => Promise<void>;
  refreshAccounts: () => Promise<void>;
  providerAccounts: (providerId: string) => Account[];
  ensureModels: (providerId: string, accountId?: string) => Promise<ModelInfo[]>;
  cachedModels: (providerId: string, accountId?: string) => ModelInfo[];
  healthFor: (providerId: string, accountId?: string) => Promise<ProviderHealth | null>;
  testAccount: (accountId: string) => Promise<AccountTest | null>;
  saveSecret: (accountId: string, secret: string) => Promise<boolean>;
  createAccount: (
    providerId: string,
    label: string,
    authKind: string,
    secret?: string,
  ) => Promise<Account | null>;
  renameAccount: (accountId: string, label: string) => Promise<void>;
  deleteAccount: (accountId: string) => Promise<void>;
  updateProvider: (
    id: string,
    patch: { name?: string; baseUrl?: string; enabled?: boolean },
  ) => Promise<void>;
  defaultSelection: () => Promise<ModelSelection | null>;
}

function key(providerId: string, accountId?: string): string {
  return `${providerId}:${accountId ?? "default"}`;
}

export const useProvidersStore = create<ProvidersState>()((set, get) => ({
  providers: [],
  accounts: [],
  models: {},
  health: {},
  loading: false,
  async load() {
    set({ loading: true });
    try {
      const [providers, accounts] = await Promise.all([
        api.listProviders(),
        api.listAccounts(),
      ]);
      set({ providers, accounts, loading: false });
      const ollama = providers.find((provider) => provider.kind === "ollama");
      const account = accounts.find((item) => item.providerId === ollama?.id);
      if (ollama) {
        void get().ensureModels(ollama.id, account?.id).catch(() => undefined);
      }
    } catch (error) {
      set({ loading: false });
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async refreshAccounts() {
    try {
      const accounts = await api.listAccounts();
      set({ accounts });
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  providerAccounts(providerId) {
    return get().accounts.filter((account) => account.providerId === providerId);
  },
  async ensureModels(providerId, accountId) {
    const cacheKey = key(providerId, accountId);
    const existing = get().models[cacheKey];
    if (existing) return existing;
    try {
      const models = await api.listModels(providerId, accountId);
      set((state) => ({ models: { ...state.models, [cacheKey]: models } }));
      return models;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return [];
    }
  },
  cachedModels(providerId, accountId) {
    return get().models[key(providerId, accountId)] ?? [];
  },
  async healthFor(providerId, accountId) {
    try {
      const health = await api.providerHealth(providerId, accountId);
      set((state) => ({
        health: { ...state.health, [key(providerId, accountId)]: health },
      }));
      return health;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return null;
    }
  },
  async testAccount(accountId) {
    try {
      const result = await api.testAccount(accountId);
      await get().refreshAccounts();
      const account = get().accounts.find((item) => item.id === accountId);
      if (account) {
        const models = await get().ensureModels(account.providerId, account.id);
        if (models.length === 0) {
          set((state) => {
            const modelsNext = { ...state.models };
            delete modelsNext[key(account.providerId, account.id)];
            return { models: modelsNext };
          });
        }
      }
      return result;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return null;
    }
  },
  async saveSecret(accountId, secret) {
    try {
      await api.updateAccountSecret(accountId, secret);
      await get().refreshAccounts();
      return true;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return false;
    }
  },
  async createAccount(providerId, label, authKind, secret) {
    try {
      const account = await api.createAccount(providerId, label, authKind, secret);
      await get().refreshAccounts();
      return account;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return null;
    }
  },
  async renameAccount(accountId, label) {
    try {
      await api.renameAccount(accountId, label);
      await get().refreshAccounts();
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async deleteAccount(accountId) {
    try {
      await api.deleteAccount(accountId);
      await get().refreshAccounts();
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async updateProvider(id, patch) {
    try {
      const provider = await api.updateProvider(id, patch);
      set((state) => ({
        providers: state.providers.map((item) => (item.id === id ? provider : item)),
      }));
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async defaultSelection() {
    const state = get();
    const provider =
      state.providers.find((item) => item.kind === "ollama" && item.enabled) ??
      state.providers.find((item) => item.enabled);
    if (!provider) return null;
    const account = state.accounts.find((item) => item.providerId === provider.id);
    if (!account) return null;
    const models = await state.ensureModels(provider.id, account.id);
    const model =
      models.find((item) => item.capabilities.includes("completion")) ?? models[0];
    if (!model) return null;
    return { providerId: provider.id, accountId: account.id, model: model.id };
  },
}));
