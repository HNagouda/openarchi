import { useState, type ReactNode } from "react";
import { readPreference, writePreference } from "../../lib/storage";
import type { Catalog, Look, ProjectView } from "../../lib/types";
import { Segmented } from "../../ui/Segmented";
import { sheetDiagrams } from "../../lib/diagrams";
import { AnalysisPanel } from "./AnalysisPanel";
import { LayersSection } from "./LayersSection";
import { RatiosPanel } from "./RatiosPanel";
import { SheetSection } from "./SheetSection";
import { ThemeSection } from "./ThemeSection";
import { ViewSection } from "./ViewSection";
import { canPlaceEye } from "../viewpoint/viewPointEdits";
import { ViewPointSection, type ViewPointControls } from "../viewpoint/ViewPointSection";

type Tab = "style" | "ratios" | "analysis";

type Props = {
  catalog: Catalog;
  project: ProjectView;
  look: Look;
  onLook: (next: Look, mergeKey?: string) => void;
  onSeal: () => void;
  onCatalogChanged: () => Promise<unknown> | void;
  viewPoint: ViewPointControls;
  /** The imported model's section, which edits the model rather than the look. */
  modelSection: ReactNode;
};

/** The right-hand panel. Every control edits the look, and every edit can be undone. */
export function Inspector({ catalog, project, look, onLook, onSeal, onCatalogChanged, viewPoint, modelSection }: Props) {
  const [tab, setTab] = useState<Tab>(() => readPreference<Tab>("inspectorTab", "style"));
  const drawn = sheetDiagrams(catalog.diagrams, look);
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
            { value: "analysis", label: "Analysis", tip: "Site and climate" },
          ]}
          onChange={changeTab}
        />
      </div>
      {tab === "ratios" && <RatiosPanel project={project} themeId={look.theme_id} selection={look.selection} />}
      {tab === "analysis" && <AnalysisPanel project={project} themeId={look.theme_id} selection={look.selection} />}
      {tab === "style" && (
        <>
          <ThemeSection
            themes={catalog.themes}
            selectedId={look.theme_id}
            onSelect={(theme_id) => onLook({ ...look, theme_id })}
            look={look}
            onCatalogChanged={onCatalogChanged}
          />
          {theme && (
            <LayersSection
              diagrams={drawn}
              isSection={look.view.mode === "section"}
              theme={theme}
              look={look}
              labels={catalog.layers}
              layersPresent={project.layers_present}
              onLook={onLook}
              onSeal={onSeal}
            />
          )}
          {canPlaceEye(drawn, look) && <ViewPointSection look={look} controls={viewPoint} onLook={onLook} />}
          {modelSection}
          <ViewSection diagrams={catalog.diagrams} look={look} onLook={onLook} onSeal={onSeal} />
          <SheetSection look={look} onLook={onLook} onSeal={onSeal} />
        </>
      )}
    </aside>
  );
}
