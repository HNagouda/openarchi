// The Analysis view of the inspector: a report on where the site is, what is
// on it and its climate. Every chart is an SVG the core drew in the sheet's
// palette, the same one the export writes, so the panel shows exactly what
// leaves the app. Each part loads on its own, because climate and rain wait
// on PVGIS and KNMI the first time while location and site are quick.

import { useCallback, useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { analysisFileName, errorText, exportAnalysis, getAnalysis, revealPath } from "../../lib/ipc";
import { fileManagerName } from "../../lib/platform";
import type { AnalysisPart, AnalysisSection, Area, ProjectView } from "../../lib/types";
import { Button } from "../../ui/Button";
import { Problem } from "../../ui/Problem";
import { Section } from "../../ui/Section";
import { useToast } from "../../ui/Toasts";
import "./analysis.css";

/** Every part in report order, with what the panel says while it loads. */
const PARTS: { part: AnalysisPart; title: string; waiting: string }[] = [
  { part: "location", title: "Location", waiting: "Looking up the address and municipality." },
  { part: "site", title: "Site", waiting: "Summing up the site." },
  { part: "climate", title: "Climate", waiting: "Fetching a typical year of weather from PVGIS. The first time takes a few seconds." },
  { part: "rain", title: "Rain", waiting: "Fetching thirty years of rain from KNMI." },
];

const SVG_FILTER = { name: "SVG page", extensions: ["svg"] };
const PDF_FILTER = { name: "PDF page", extensions: ["pdf"] };

/** What the intro says for the whole site. On a sheet area the Site part's source line says what it describes. */
const INTRO_WHOLE_SITE = "Location, site and climate, drawn in the sheet's palette.";

type Props = { project: ProjectView; themeId: string; selection?: Area | null };

export function AnalysisPanel({ project, themeId, selection = null }: Props) {
  const toast = useToast();

  const exportTo = async (sectionId: string | null) => {
    try {
      const path = await save({
        defaultPath: await analysisFileName(sectionId),
        filters: sectionId ? [SVG_FILTER, PDF_FILTER] : [PDF_FILTER, SVG_FILTER],
      });
      if (!path) return;
      const written = await exportAnalysis(themeId, selection, sectionId, path);
      const fileName = written.split(/[\\/]/).pop() ?? written;
      toast(`Exported ${fileName}`, { action: { label: `Show in ${fileManagerName}`, run: () => revealPath(written) } });
    } catch (reason) {
      toast(`Couldn't export the analysis. ${errorText(reason)}`, { isError: true });
    }
  };

  if (!project.has_scene) {
    return <p className="analysis-empty inspector-note">The analysis appears once the site has been fetched.</p>;
  }

  return (
    <div className="analysis">
      <div className="analysis-toolbar">
        {!selection && <p className="analysis-intro">{INTRO_WHOLE_SITE}</p>}
        <Button icon="export" variant="secondary" tip="Save the whole analysis as one PDF or SVG page" onClick={() => exportTo(null)}>
          Export
        </Button>
      </div>
      {PARTS.map((entry) => (
        <PartView
          key={entry.part}
          {...entry}
          themeId={themeId}
          // Only the site part follows the area, so a new area leaves the others alone.
          selection={entry.part === "site" ? selection : null}
          fetchedAt={project.fetched_at}
          onExport={exportTo}
        />
      ))}
    </div>
  );
}

type PartProps = {
  part: AnalysisPart;
  title: string;
  waiting: string;
  themeId: string;
  selection: Area | null;
  fetchedAt: string | null;
  onExport: (sectionId: string) => void;
};

function PartView({ part, title, waiting, themeId, selection, fetchedAt, onExport }: PartProps) {
  const { sections, error, retry } = useAnalysisPart(part, themeId, selection, fetchedAt);

  if (error && !sections) {
    return (
      <Section id={`analysis.${part}`} title={title}>
        <Problem title={`The ${title.toLowerCase()} couldn't be fetched.`} detail={error}>
          <Button variant="secondary" onClick={retry}>
            Try again
          </Button>
        </Problem>
      </Section>
    );
  }
  if (!sections) {
    return (
      <Section id={`analysis.${part}`} title={title}>
        <div className="analysis-waiting" role="status">
          <div className="analysis-placeholder" aria-hidden="true" />
          <p>{waiting}</p>
        </div>
      </Section>
    );
  }

  return (
    <>
      {sections.map((section) => (
        <SectionView key={section.id} section={section} onExport={() => onExport(section.id)} />
      ))}
    </>
  );
}

function SectionView({ section, onExport }: { section: AnalysisSection; onExport: () => void }) {
  const exportButton = <Button icon="export" tip={`Export ${section.title.toLowerCase()} as SVG or PDF`} tipSide="left" onClick={onExport} />;
  return (
    <Section id={`analysis.${section.id}`} title={section.title} aside={exportButton}>
      {section.source && <p className="analysis-source">{section.source}</p>}
      {section.facts.length > 0 && (
        <dl className="analysis-facts">
          {section.facts.map((fact) => (
            <div key={fact.label}>
              <dt>{fact.label}</dt>
              <dd>{fact.value}</dd>
            </div>
          ))}
        </dl>
      )}
      {section.figures.map((figure) => (
        // The core's own SVG, drawn from data it fetched; no user text goes into it unescaped.
        <div key={figure.id} className="analysis-figure" dangerouslySetInnerHTML={{ __html: figure.svg }} />
      ))}
      {section.caption && <p className="analysis-caption">{section.caption}</p>}
      {section.notes.length > 0 && (
        <ul className="analysis-notes">
          {section.notes.map((note) => (
            <li key={note}>{note}</li>
          ))}
        </ul>
      )}
      {section.warnings.map((warning) => (
        <p key={warning} className="inspector-note">
          {warning}
        </p>
      ))}
    </Section>
  );
}

/** One part for the open site in `themeId`, or for `selection` when the part
    follows the sheet's area. A palette or area change keeps the old charts on
    screen until the new ones arrive, so nothing flickers. */
function useAnalysisPart(part: AnalysisPart, themeId: string, selection: Area | null, fetchedAt: string | null) {
  const [sections, setSections] = useState<AnalysisSection[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const selectionKey = JSON.stringify(selection);

  useEffect(() => {
    let isCurrent = true;
    getAnalysis(part, themeId, selection)
      .then((result) => {
        if (!isCurrent) return;
        setSections(result);
        setError(null);
      })
      .catch((reason) => {
        if (!isCurrent) return;
        setSections(null);
        setError(errorText(reason));
      });
    return () => {
      isCurrent = false;
    };
    // `selectionKey` stands for `selection`, which is a new object on every render.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [part, themeId, selectionKey, fetchedAt, attempt]);

  const retry = useCallback(() => {
    setError(null);
    setAttempt((count) => count + 1);
  }, []);

  return { sections, error, retry };
}
