// The first-launch tour. Bump TOUR_EDITION when a release adds something
// worth showing, and returning users see the tour once more.

import { useState, type ReactNode } from "react";
import { isTyping, useKeyDown } from "../lib/keys";
import { writePreference } from "../lib/storage";
import { Button, Keys } from "../ui/Button";
import { Dialog } from "../ui/Dialog";
import { TourArt } from "./TourArt";

export const TOUR_EDITION = 1;

type Step = { title: string; art: "site" | "diagram" | "theme" | "export" | "assistant"; body: ReactNode };

const STEPS: Step[] = [
  {
    title: "Draw a site",
    art: "site",
    body: (
      <>
        Search an address in the Netherlands, then draw the site as a circle <Keys shortcut="c" />, rectangle{" "}
        <Keys shortcut="r" /> or polygon <Keys shortcut="p" />. openarchi fetches the buildings, streets, water and
        green inside it.
      </>
    ),
  },
  {
    title: "Pick a diagram",
    art: "diagram",
    body: (
      <>
        Each diagram answers one question about the site, like what is built and what is open. The strip along the
        bottom shows the whole series; keys <Keys shortcut="1" /> to <Keys shortcut="9" /> switch between them.
      </>
    ),
  },
  {
    title: "Pick a palette",
    art: "theme",
    body: (
      <>
        A palette styles the whole series at once, so the sheets read as a set. Step through palettes with{" "}
        <Keys shortcut="[" /> and <Keys shortcut="]" />, change any layer's colour in the inspector, and undo anything
        with <Keys shortcut="mod+z" />.
      </>
    ),
  },
  {
    title: "Export",
    art: "export",
    body: (
      <>
        <Keys shortcut="mod+e" /> exports this diagram or the whole series as PDF, SVG, PNG or DXF. Projects save
        themselves, and every earlier version is kept as a backup.
      </>
    ),
  },
  {
    title: "Connect an assistant",
    art: "assistant",
    body: (
      <>
        An assistant like Claude can make diagrams for you through openarchi's MCP server, and its changes appear
        here as they happen. Help › Connect an Assistant shows how.
      </>
    ),
  },
];

export function Tour({ onClose }: { onClose: () => void }) {
  const [index, setIndex] = useState(0);
  const step = STEPS[index];
  const isLast = index === STEPS.length - 1;

  const close = () => {
    writePreference("tourEdition", TOUR_EDITION);
    onClose();
  };

  const next = () => (isLast ? close() : setIndex(index + 1));
  const back = () => setIndex(Math.max(0, index - 1));

  useKeyDown((event) => {
    if (isTyping(event.target) || event.repeat) return;
    if (event.key === "ArrowRight" || event.key === "Enter") next();
    if (event.key === "ArrowLeft") back();
  });

  return (
    <Dialog
      title="Welcome to openarchi"
      width={440}
      onClose={close}
      footer={
        <div className="tour-footer">
          <div className="tour-dots" aria-label={`Step ${index + 1} of ${STEPS.length}`}>
            {STEPS.map((entry, dot) => (
              <button
                type="button"
                key={entry.title}
                className={dot === index ? "tour-dot is-current" : "tour-dot"}
                onClick={() => setIndex(dot)}
                aria-label={entry.title}
              />
            ))}
          </div>
          <div className="tour-buttons">
            {index > 0 ? (
              <Button variant="secondary" onClick={back}>
                Back
              </Button>
            ) : (
              <Button onClick={close}>Skip</Button>
            )}
            <Button variant="primary" onClick={next}>
              {isLast ? "Start" : "Next"}
            </Button>
          </div>
        </div>
      }
    >
      <div className="tour-step" key={step.title}>
        <TourArt kind={step.art} />
        <span className="tour-count tabular">
          {index + 1} of {STEPS.length}
        </span>
        <h3>{step.title}</h3>
        <p>{step.body}</p>
      </div>
    </Dialog>
  );
}
