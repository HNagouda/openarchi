import type { ButtonHTMLAttributes, ReactNode } from "react";
import { shortcutLabel } from "../lib/platform";
import { Icon, type IconName } from "./Icon";

type Variant = "plain" | "primary" | "secondary" | "ghost";

type Props = ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: Variant;
  icon?: IconName;
  /** Shown in the tooltip next to `tip`, platform-correct: "mod+e" -> "⌘E". */
  shortcut?: string;
  tip?: string;
  tipSide?: "below" | "above" | "left";
  isActive?: boolean;
  children?: ReactNode;
};

/** Every button in the app. A `tip` shows after a short hover delay, with its shortcut. */
export function Button({
  variant = "plain",
  icon,
  shortcut,
  tip,
  tipSide = "below",
  isActive = false,
  className = "",
  children,
  ...rest
}: Props) {
  const isIconOnly = icon !== undefined && children === undefined;
  const classes = ["button", `button-${variant}`, isIconOnly ? "button-icon" : "", isActive ? "is-active" : "", className];
  return (
    <button
      type="button"
      className={classes.filter(Boolean).join(" ")}
      data-tip={tip}
      data-tip-key={shortcut ? shortcutLabel(shortcut) : undefined}
      data-tip-side={tip ? tipSide : undefined}
      aria-label={isIconOnly ? tip : undefined}
      {...rest}
    >
      {icon && <Icon name={icon} />}
      {children !== undefined && <span className="button-label">{children}</span>}
    </button>
  );
}

/** A key cap, for shortcut lists and the tour. */
export function Keys({ shortcut }: { shortcut: string }) {
  return <kbd className="keys">{shortcutLabel(shortcut)}</kbd>;
}
