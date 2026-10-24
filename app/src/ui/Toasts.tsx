// Small messages at the bottom centre of the workspace, for changes that
// happen out of view: a delete, an export, an assistant's edit.

import { createContext, useCallback, useContext, useEffect, useRef, useState, type ReactNode } from "react";

// Long enough to reach for Undo, short enough not to linger.
const TOAST_MS = 5000;

type Action = { label: string; run: () => void };
type Toast = { id: number; message: string; action?: Action; isError?: boolean };
type Show = (message: string, options?: { action?: Action; isError?: boolean }) => void;

const ToastContext = createContext<Show>(() => undefined);

export const useToast = () => useContext(ToastContext);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<Toast[]>([]);
  const nextId = useRef(1);

  const dismiss = useCallback((id: number) => setToasts((current) => current.filter((toast) => toast.id !== id)), []);

  const show = useCallback<Show>((message, options = {}) => {
    const id = nextId.current++;
    // One toast at a time reads calmer than a stack.
    setToasts([{ id, message, ...options }]);
  }, []);

  return (
    <ToastContext.Provider value={show}>
      {children}
      <div className="toasts" aria-live="polite">
        {toasts.map((toast) => (
          <ToastView key={toast.id} toast={toast} onDismiss={() => dismiss(toast.id)} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

function ToastView({ toast, onDismiss }: { toast: Toast; onDismiss: () => void }) {
  const [isHovered, setHovered] = useState(false);

  useEffect(() => {
    if (isHovered) return;
    const timer = window.setTimeout(onDismiss, TOAST_MS);
    return () => window.clearTimeout(timer);
  }, [isHovered, onDismiss]);

  return (
    <div
      className={toast.isError ? "toast is-error" : "toast"}
      role="status"
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => setHovered(false)}
    >
      <span className="toast-message">{toast.message}</span>
      {toast.action && (
        <button
          type="button"
          className="toast-action"
          onClick={() => {
            toast.action?.run();
            onDismiss();
          }}
        >
          {toast.action.label}
        </button>
      )}
    </div>
  );
}
