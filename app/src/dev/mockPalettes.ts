// The palette commands for the dev-only IPC mock. app_fixtures writes one
// palette made from five swatches, "westermarkt-sample", with every diagram
// drawn in it. The mock cannot mix a new theme: a draft duplicated from a
// palette checks and previews as that palette, any other as the sample, and
// saving a new one renames the sample.

import type { PaletteDraft } from "../lib/palette";
import type { Catalog, Look, Theme } from "../lib/types";

const FIXTURES = "/dev-fixtures/palette";
const SAMPLE_ID = "westermarkt-sample";

type Args = Record<string, unknown>;
type RenderPreview = (look: Look) => Promise<string>;

// What the user did to their palettes this session, kept until the page reloads.
const names = new Map<string, string>();
const deleted = new Set<string>();

const json = (file: string) => fetch(`${FIXTURES}/${file}`).then((response) => response.json());

/** The catalog with the session's renames and deletions applied. */
export function withPaletteEdits(catalog: Catalog): Catalog {
  const themes = catalog.themes
    .filter((theme) => !deleted.has(theme.id))
    .map((theme) => ({ ...theme, name: names.get(theme.id) ?? theme.name }));
  return { ...catalog, themes };
}

function sampleTheme(catalog: Catalog): Theme {
  const theme = catalog.themes.find((entry) => entry.id === SAMPLE_ID);
  if (!theme) throw new Error("The dev fixtures have no sample palette. Run the app_fixtures example again.");
  return { ...theme, name: names.get(SAMPLE_ID) ?? theme.name };
}

/** The answer to a palette command, or undefined for any other command. */
export async function answerPalette(command: string, args: Args, catalog: Catalog, renderPreview: RenderPreview) {
  if (command === "import_palette") return json("swatches.json");
  if (command === "check_palette") {
    const basedOn = (args.draft as PaletteDraft).based_on;
    return json(basedOn ? `check--${basedOn}.json` : "check.json");
  }
  if (command === "render_palette_preview") {
    const themeId = (args.draft as PaletteDraft).based_on ?? SAMPLE_ID;
    return renderPreview({ ...(args.look as Look), theme_id: themeId });
  }
  if (command === "palette_draft") {
    const draft: PaletteDraft = await json(`draft--${args.themeId}.json`);
    return { ...draft, name: String(args.name) };
  }
  if (command === "save_palette") {
    names.set(SAMPLE_ID, (args.draft as PaletteDraft).name);
    deleted.delete(SAMPLE_ID);
    return sampleTheme(catalog);
  }
  if (command === "rename_palette") {
    names.set(String(args.id), String(args.name));
    return null;
  }
  if (command === "delete_palette") {
    deleted.add(String(args.id));
    return String(args.id);
  }
  if (command === "restore_palette") {
    deleted.delete(String(args.kept));
    return sampleTheme(catalog);
  }
  return undefined;
}
