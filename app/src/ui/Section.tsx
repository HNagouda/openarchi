import { useState, type ReactNode } from "react";
import { readPreference, writePreference } from "../lib/storage";
import { Icon } from "./Icon";

/** An inspector section with a disclosure header. Its open state is remembered per machine. */
export function Section({ id, title, children, aside }: { id: string; title: string; children: ReactNode; aside?: ReactNode }) {
  const [isOpen, setOpen] = useState(() => readPreference(`section.${id}`, true));

  const toggle = () => {
    setOpen(!isOpen);
    writePreference(`section.${id}`, !isOpen);
  };

  return (
    <section className={isOpen ? "section is-open" : "section"}>
      <header className="section-header">
        <button type="button" className="section-toggle" onClick={toggle} aria-expanded={isOpen}>
          <span className="section-chevron">
            <Icon name="chevronRight" size={12} />
          </span>
          {title}
        </button>
        {aside && <div className="section-aside">{aside}</div>}
      </header>
      {isOpen && <div className="section-body">{children}</div>}
    </section>
  );
}
