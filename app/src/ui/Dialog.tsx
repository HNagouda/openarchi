import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";

type Props = {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  /** Pixels, or a CSS length such as "min(1600px, 100vw)" for a dialog that sizes to the window. */
  width?: number | string;
  /** Sheets drop from the top bar like a macOS sheet; panels sit centred. */
  placement?: "sheet" | "center";
};

/** A modal over a soft scrim. Escape and a click on the scrim close it.
 * Drawn into the body, because a dialog opened from inside a container (the
 * inspector is one, for its container queries) would otherwise be laid out
 * and clipped inside it. */
export function Dialog({ title, onClose, children, footer, width = 400, placement = "center" }: Props) {
  const panel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.stopPropagation();
        onClose();
      }
    };
    window.addEventListener("keydown", listener, true);
    panel.current?.focus();
    return () => window.removeEventListener("keydown", listener, true);
  }, [onClose]);

  return createPortal(
    <div className={`scrim scrim-${placement}`} data-modal-open onMouseDown={onClose}>
      <div
        ref={panel}
        className={`dialog dialog-${placement}`}
        style={{ width }}
        role="dialog"
        aria-label={title}
        tabIndex={-1}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="dialog-header">
          <h2>{title}</h2>
        </header>
        <div className="dialog-body">{children}</div>
        {footer && <footer className="dialog-footer">{footer}</footer>}
      </div>
    </div>,
    document.body,
  );
}
