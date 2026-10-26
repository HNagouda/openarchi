// A thin bar under a line of text, for work that takes a while: filled to
// `done` of `total`, or sweeping while the count is not known yet.

import { PERCENT } from "../lib/units";

// How much of the track the sweeping bar covers while the count is unknown.
const WAITING_WIDTH = "30%";

type Props = { label: string; done: number | null; total: number | null };

export function ProgressBar({ label, done, total }: Props) {
  const isCounted = done !== null && total !== null;
  return (
    <div className="progress" role="progressbar" aria-label={label} aria-valuenow={done ?? undefined} aria-valuemax={total ?? undefined}>
      <div className="progress-row">
        <span>{label}</span>
        {isCounted && (
          <span className="tabular progress-count">
            {done} of {total}
          </span>
        )}
      </div>
      <div className="progress-track">
        <div
          className={isCounted ? "progress-fill" : "progress-fill is-waiting"}
          style={{ width: isCounted ? `${(done / Math.max(1, total)) * PERCENT}%` : WAITING_WIDTH }}
        />
      </div>
    </div>
  );
}
