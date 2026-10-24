import { useState } from "react";
import { readPreference, writePreference } from "../../lib/storage";
import type { Catalog, Look, ProjectView } from "../../lib/types";
import { Segmented } from "../../ui/Segmented";
import { AnalysisPanel } from "./AnalysisPanel";
import { LayersSection } from "./LayersSection";
import { RatiosPanel } from "./RatiosPanel";
import { SheetSection } from "./SheetSection";
import { ThemeSection } from "./ThemeSection";
import { ViewSection } from "./ViewSection";

type Tab = "style" | "ratios" | "analysis";

type Props = {
  catalog: Catalog;
  project: ProjectView;
  look: Look;
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
};

/** The right-hand panel. Every control edits the look, and every edit can be undone. */
export function Inspector({ catalog, project, look, onLook, onSeal }: Props) {
  const [tab, setTab] = useState<Tab>(() => readPreference<Tab>("inspectorTab", "style"));
  const diagram = catalog.diagrams.find((entry) => entry.id === look.diagram_id);
  const theme = catalog.themes.find((entry) => entry.id === look.theme_id);

  const changeTab = (next: Tab) => {
    setTab(next);
    writePreference("inspectorTab", next);
  };

  return (
    <aside className="inspector">
      <div className="inspector-tabs">
        <Segmented
          label="Inspector"
          value={tab}
          options={[
            { value: "style", label: "Style" },
            { value: "ratios", label: "Ratios", tip: "How the site divides up, in numbers" },
            { value: "analysis", label: "Analysis", tip: "Location, site and climate, with design notes" },
          ]}
          onChange={changeTab}
        />
      </div>
      {tab === "ratios" && <RatiosPanel project={project} themeId={look.theme_id} />}
      {tab === "analysis" && <AnalysisPanel project={project} themeId={look.theme_id} />}
      {tab === "style" && (
        <>
          <ThemeSection themes={catalog.themes} selectedId={look.theme_id} onSelect={(theme_id) => onLook({ ...look, theme_id })} />
          {diagram && theme && (
            <LayersSection
              diagram={diagram}
              theme={theme}
              look={look}
              labels={catalog.layers}
              layersPresent={project.layers_present}
              onLook={onLook}
              onSeal={onSeal}
            />
          )}
          <ViewSection diagrams={catalog.diagrams} look={look} onLook={onLook} onSeal={onSeal} />
          <SheetSection look={look} onLook={onLook} onSeal={onSeal} />
        </>
      )}
    </aside>
  );
}
