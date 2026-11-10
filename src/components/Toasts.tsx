import { CircleAlert, CircleCheck, Info, X } from "lucide-react";

import { useUiStore } from "@/stores/ui";

export function Toasts() {
  const toasts = useUiStore((state) => state.toasts);
  const dismiss = useUiStore((state) => state.dismiss);
  if (toasts.length === 0) return null;
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((toast) => (
        <div key={toast.id} className={`toast ${toast.kind}`}>
          {toast.kind === "error" ? (
            <CircleAlert size={15} />
          ) : toast.kind === "success" ? (
            <CircleCheck size={15} />
          ) : (
            <Info size={15} />
          )}
          <span>{toast.text}</span>
          <button onClick={() => dismiss(toast.id)} aria-label="Dismiss">
            <X size={13} />
          </button>
        </div>
      ))}
    </div>
  );
}
