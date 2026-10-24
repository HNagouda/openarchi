// Whether the draft reads on its paper, from the core's WCAG 2.1 check. Text
// or buildings below their ratio block saving; a faint accent only warns.

import { Icon } from "../../ui/Icon";

type Props = { warnings: string[]; isReadable: boolean };

export function Readability({ warnings, isReadable }: Props) {
  if (warnings.length === 0) {
    return (
      <p className="palette-readable">
        <Icon name="check" size={14} />
        Text and buildings read clearly on this paper (WCAG 2.1 contrast).
      </p>
    );
  }

  return (
    <ul className={isReadable ? "palette-warnings" : "palette-warnings is-blocking"} aria-label="Contrast">
      {warnings.map((warning) => (
        <li key={warning}>{warning}</li>
      ))}
    </ul>
  );
}
