import { create } from "zustand";

import { api, errorMessage } from "@/lib/api";
import type { AppInfo, BackupInfo } from "@/types";

import { useChatStore } from "./chat";
import { useProjectsStore } from "./projects";
import { useProvidersStore } from "./providers";
import { useUiStore } from "./ui";

type Theme = "system" | "light" | "dark";

interface AppState {
  info: AppInfo | null;
  settings: Record<string, string>;
  ready: boolean;
  loading: boolean;
  saveSettings: (entries: Record<string, string>) => Promise<boolean>;
  setTheme: (theme: Theme) => Promise<void>;
  completeOnboarding: () => Promise<void>;
  backup: () => Promise<BackupInfo | null>;
  init: () => Promise<void>;
}

let systemListenerAttached = false;

export function resolveTheme(theme: string | undefined): "light" | "dark" {
  if (theme === "light" || theme === "dark") return theme;
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function applyTheme(theme: string | undefined): void {
  const resolved = resolveTheme(theme);
  document.documentElement.dataset.theme = resolved;
  document.documentElement.style.colorScheme = resolved;
}

export function applyPlatform(platform: string | undefined): void {
  if (platform === "macos" || platform === "windows" || platform === "linux") {
    document.documentElement.dataset.platform = platform;
  }
}

export function watchSystemTheme(getTheme: () => string | undefined): void {
  if (systemListenerAttached || !window.matchMedia) return;
  systemListenerAttached = true;
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  media.addEventListener("change", () => {
    if ((getTheme() ?? "system") === "system") applyTheme("system");
  });
}

export const useAppStore = create<AppState>()((set, get) => ({
  info: null,
  settings: {},
  ready: false,
  loading: false,
  async init() {
    if (get().loading || get().ready) return;
    set({ loading: true });
    try {
      const [info, settings] = await Promise.all([api.appInfo(), api.getSettings()]);
      set({ info, settings });
      applyTheme(settings.theme);
      applyPlatform(info.platform);
      watchSystemTheme(() => get().settings.theme);
      await Promise.all([
        useProvidersStore.getState().load(),
        useProjectsStore.getState().load(),
        useChatStore.getState().loadConversations(),
      ]);
      set({ ready: true, loading: false });
      if (settings.onboardingComplete !== "true") {
        useUiStore.getState().navigate({ name: "onboarding" });
      }
    } catch (error) {
      set({ loading: false });
      useUiStore.getState().toast(errorMessage(error), "error");
    }
  },
  async saveSettings(entries) {
    try {
      const settings = await api.updateSettings(entries);
      set({ settings });
      if (entries.theme) applyTheme(entries.theme);
      return true;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return false;
    }
  },
  async setTheme(theme) {
    await get().saveSettings({ theme });
  },
  async completeOnboarding() {
    const saved = await get().saveSettings({ onboardingComplete: "true" });
    if (saved) useUiStore.getState().navigate({ name: "chat" });
  },
  async backup() {
    try {
      const result = await api.backupDatabase();
      useUiStore.getState().toast(`Backup saved to ${result.path}`, "success");
      return result;
    } catch (error) {
      useUiStore.getState().toast(errorMessage(error), "error");
      return null;
    }
  },
}));
