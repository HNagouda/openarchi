// The Ratios view of the inspector: how the site divides up, group by group,
// as bars or pies. The numbers and colours come from the core, so they match
// the diagrams and the exported CSV and SVG.

import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { errorText, exportRatios, getRatios, ratiosFileName, revealPath } from "../../lib/ipc";
import { formatArea } from "../../lib/format";
import { fileManagerName } from "../../lib/platform";
import { readPreference, writePreference } from "../../lib/storage";
import type { ChartStyle, ProjectView, RatioGroup, RatioRow, RatioUnit, Ratios } from "../../lib/types";
import { Button } from "../../ui/Button";
import { Problem } from "../../ui/Problem";
import { Section } from "../../ui/Section";
import { Segmented } from "../../ui/Segmented";
import { useToast } from "../../ui/Toasts";
import "./ratios.css";

const PIE_RADIUS = 44;
const FULL_TURN = 2 * Math.PI;

const count = new Intl.NumberFormat("en-GB", { maximumFractionDigits: 0 });
const percent = new Intl.NumberFormat("en-GB", { style: "percent", minimumFractionDigits: 1, maximumFractionDigits: 1 });
const ratio = new Intl.NumberFormat("en-GB", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

type Props = { project: ProjectView; themeId: string };

export function RatiosPanel({ project, themeId }: Props) {
  const [style, setStyle] = useState<ChartStyle>(() => readPreference<ChartStyle>("ratios.style", "bar"));
  const { ratios, error } = useRatios(themeId, project.has_scene, project.fetched_at);
  const toast = useToast();

  const changeStyle = (next: ChartStyle) => {
    setStyle(next);
    writePreference("ratios.style", next);
  };

  const exportFile = async () => {
    try {
      const path = await save({
        defaultPath: await ratiosFileName(),
        filters: [
          { name: "CSV table", extensions: ["csv"] },
          { name: "SVG charts", extensions: ["svg"] },
        ],
      });
      if (!path) return;
      const written = await exportRatios(themeId, style, path);
      const fileName = written.split(/[\\/]/).pop() ?? written;
      toast(`Exported ${fileName}`, { action: { label: `Show in ${fileManagerName}`, run: () => revealPath(written) } });
    } catch (reason) {
      toast(`Couldn't export the ratios. ${errorText(reason)}`, { isError: true });
    }
  };

  if (!project.has_scene) {
    return <p className="ratios-empty inspector-note">Ratios appear once the site has been fetched.</p>;
  }

  return (
    <div className="ratios">
      <div className="ratios-toolbar">
        <Segmented
          label="Chart"
          value={style}
          options={[
            { value: "bar", label: "Bars" },
            { value: "pie", label: "Pies" },
          ]}
          onChange={changeStyle}
        />
        <Button icon="export" variant="secondary" tip="Save as a CSV table or SVG charts" disabled={!ratios} onClick={exportFile}>
          Export
        </Button>
      </div>
      {error && (
        <div className="ratios-empty">
          <Problem title="The ratios couldn't be worked out." detail={error} />
        </div>
      )}
      {ratios && <Summary ratios={ratios} />}
      {ratios?.groups.map((group) => (
        <Section key={group.id} id={`ratios.${group.id}`} title={group.title} aside={<span className="tabular">{totalText(group)}</span>}>
          {style === "bar" ? <Bars group={group} /> : <Pie group={group} />}
          {group.note && <p className="inspector-note">{group.note}</p>}
        </Section>
      ))}
    </div>
  );
}

/** Ratios for the open site in `themeId`, again whenever the theme or the fetched site changes. */
function useRatios(themeId: string, hasScene: boolean, fetchedAt: string | null) {
  const [ratios, setRatios] = useState<Ratios | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!hasScene) return;
    let isCurrent = true;
    getRatios(themeId)
      .then((result) => {
        if (!isCurrent) return;
        setRatios(result);
        setError(null);
      })
      .catch((reason) => isCurrent && setError(errorText(reason)));
    return () => {
      isCurrent = false;
    };
  }, [themeId, hasScene, fetchedAt]);

  return { ratios, error };
}

function Summary({ ratios }: { ratios: Ratios }) {
  return (
    <dl className="ratios-summary">
      <div>
        <dt>Site</dt>
        <dd className="tabular">{formatArea(ratios.site_area_m2)}</dd>
      </div>
      <div>
        <dt>Buildings</dt>
        <dd className="tabular">{count.format(ratios.building_count)}</dd>
      </div>
      <div data-tip="Building footprint over site area" data-tip-side="above">
        <dt>Coverage</dt>
        <dd className="tabular">{percent.format(ratios.coverage)}</dd>
      </div>
      <div data-tip="Estimated floor area over site area" data-tip-side="above">
        <dt>Floor area ratio</dt>
        <dd className="tabular">{ratio.format(ratios.floor_area_ratio)}</dd>
      </div>
    </dl>
  );
}

function Bars({ group }: { group: RatioGroup }) {
  return (
    <ul className="ratio-rows">
      {group.rows.map((row) => (
        <li key={row.label} className="ratio-bar-row">
          <RowText row={row} unit={group.unit} />
          <span className="ratio-track">
            <span className="ratio-bar" style={{ width: `${row.share * 100}%`, background: row.fill }} />
          </span>
        </li>
      ))}
    </ul>
  );
}

function Pie({ group }: { group: RatioGroup }) {
  let start = 0;
  const slices = group.rows.map((row) => {
    const end = start + row.share * FULL_TURN;
    const slice = { row, path: slicePath(start, end) };
    start = end;
    return slice;
  });

  return (
    <div className="ratio-pie">
      <svg viewBox={`${-PIE_RADIUS} ${-PIE_RADIUS} ${2 * PIE_RADIUS} ${2 * PIE_RADIUS}`} aria-hidden="true">
        {slices.map(({ row, path }) => (
          <path key={row.label} d={path} fill={row.fill} className="ratio-slice" />
        ))}
      </svg>
      <ul className="ratio-rows">
        {group.rows.map((row) => (
          <li key={row.label}>
            <RowText row={row} unit={group.unit} />
          </li>
        ))}
      </ul>
    </div>
  );
}

function RowText({ row, unit }: { row: RatioRow; unit: RatioUnit }) {
  return (
    <span className="ratio-row-text">
      <span className="ratio-swatch" style={{ background: row.fill }} />
      <span className="ratio-label">{row.label}</span>
      <span className="ratio-share tabular">{percent.format(row.share)}</span>
      <span className="ratio-value tabular">{valueText(row.value, unit)}</span>
    </span>
  );
}

/** A wedge from `start` to `end` radians, clockwise from twelve o'clock, as the core draws it. */
function slicePath(start: number, end: number): string {
  if (end - start >= FULL_TURN - 1e-9) {
    return `M${-PIE_RADIUS} 0a${PIE_RADIUS} ${PIE_RADIUS} 0 1 0 ${2 * PIE_RADIUS} 0a${PIE_RADIUS} ${PIE_RADIUS} 0 1 0 ${-2 * PIE_RADIUS} 0Z`;
  }
  const point = (angle: number) => `${PIE_RADIUS * Math.sin(angle)} ${-PIE_RADIUS * Math.cos(angle)}`;
  const isLarge = end - start > Math.PI ? 1 : 0;
  return `M0 0L${point(start)}A${PIE_RADIUS} ${PIE_RADIUS} 0 ${isLarge} 1 ${point(end)}Z`;
}

function valueText(value: number, unit: RatioUnit): string {
  return unit === "square_metres" ? formatArea(value) : count.format(value);
}

function totalText(group: RatioGroup): string {
  if (group.unit === "square_metres") return formatArea(group.total);
  return `${count.format(group.total)} ${group.unit}`;
}
