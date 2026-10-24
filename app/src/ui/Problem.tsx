import type { ReactNode } from "react";

/** A calm inline message for something that did not work, with an optional way forward. */
export function Problem({ title, detail, children }: { title: string; detail?: string; children?: ReactNode }) {
  return (
    <div className="problem" role="alert">
      <div className="problem-mark" aria-hidden="true" />
      <div className="problem-text">
        <p className="problem-title">{title}</p>
        {detail && <p className="problem-detail">{detail}</p>}
        {children && <div className="problem-actions">{children}</div>}
      </div>
    </div>
  );
}
