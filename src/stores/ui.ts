import { create } from "zustand";

export type View =
  | { name: "chat" }
  | { name: "projects" }
  | { name: "project"; id: string }
  | { name: "tasks" }
  | { name: "providers" }
  | { name: "settings" }
  | { name: "onboarding" };

export interface Toast {
  id: string;
  text: string;
  kind: "info" | "error" | "success";
}

interface UiState {
  view: View;
  toasts: Toast[];
  navigate: (view: View) => void;
  toast: (text: string, kind?: Toast["kind"]) => void;
  dismiss: (id: string) => void;
}

function makeId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;
}

export const useUiStore = create<UiState>()((set) => ({
  view: { name: "chat" },
  toasts: [],
  navigate: (view) => set({ view }),
  toast: (text, kind = "info") => {
    const id = makeId();
    set((state) => ({ toasts: [...state.toasts, { id, text, kind }] }));
    setTimeout(() => {
      set((state) => ({ toasts: state.toasts.filter((item) => item.id !== id) }));
    }, 5000);
  },
  dismiss: (id) =>
    set((state) => ({ toasts: state.toasts.filter((item) => item.id !== id) })),
}));
