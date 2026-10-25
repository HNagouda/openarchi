// A colour swatch that opens a picker in a popover under it. Closing the
// popover remembers the colour for the recent row and ends the undo step.

import { useRef, useState, type CSSProperties, type ReactNode } from "react";
import { ColorPopover } from "./ColorPopover";
import { rememberColour } from "./colour";

type Props = {
  /** The colour as painted, "#RRGGBB"; null shows an empty, struck-through swatch. */
  colour: string | null;
  /** Names the swatch and its popover for screen readers: "Text colour". */
  label: string;
  tip?: string;
  className?: string;
  style?: CSSProperties;
  /** Called when the popover closes: the end of one undo step. */
  onSeal?: () => void;
  /** The picker shown in the popover. */
  children: ReactNode;
};

export function ColorSwatch({ colour, label, tip, className = "", style, onSeal, children }: Props) {
  const [isOpen, setOpen] = useState(false);
  const swatch = useRef<HTMLButtonElement>(null);

  const close = () => {
    setOpen(false);
    if (colour) rememberColour(colour);
    onSeal?.();
  };

  return (
    <>
      <button
        ref={swatch}
        type="button"
        className={["swatch", colour ? "" : "is-empty", className].filter(Boolean).join(" ")}
        style={{ background: colour ?? undefined, ...style }}
        data-tip={tip}
        aria-label={label}
        aria-expanded={isOpen}
        onClick={() => (isOpen ? close() : setOpen(true))}
      />
      {isOpen && (
        <ColorPopover anchor={swatch.current} label={label} onClose={close}>
          {children}
        </ColorPopover>
      )}
    </>
  );
}
