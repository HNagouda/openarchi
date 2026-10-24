// The user's own palettes: the shapes of crates/core/src/palette.rs, the
// commands in src-tauri/src/commands/palettes.rs, and the text a palette is
// copied out as.

import { invoke } from "@tauri-apps/api/core";
import type { Look, Palette, Theme } from "./types";

export type PaletteSwatch = { hex: string; name?: string | null };

/** The roles a palette fills from its swatches, in the order the editor lists them. */
export const ROLES = ["paper", "ink", "figure", "accent", "water", "green"] as const;
export type Role = (typeof ROLES)[number];

/** Which swatch the user pinned to each role, by index. Missing lets the rule choose. */
export type RoleChoice = Partial<Record<Role, number | null>>;

export type PaletteDraft = {
  name: string;
  swatches: PaletteSwatch[];
  roles: RoleChoice;
  typography: Theme["typography"] | null;
  /** The theme it was duplicated from. */
  based_on: string | null;
};

export type RoleColour = { role: Role; hex: string; swatch: number | null; is_pinned: boolean; reason: string };

export type PaletteCheck = { theme: Theme & { palette: Palette }; roles: RoleColour[]; warnings: string[] };

export const importPalette = (text: string) => invoke<PaletteSwatch[]>("import_palette", { text });
export const checkPalette = (draft: PaletteDraft) => invoke<PaletteCheck>("check_palette", { draft });
export const renderPalettePreview = (look: Look, draft: PaletteDraft) =>
  invoke<string>("render_palette_preview", { look, draft });
export const paletteDraft = (themeId: string, name: string) => invoke<PaletteDraft>("palette_draft", { themeId, name });
/** Without an id, a new palette with an id made from its name. */
export const savePalette = (draft: PaletteDraft, id: string | null) => invoke<Theme>("save_palette", { draft, id });
export const renamePalette = (id: string, name: string) => invoke<void>("rename_palette", { id, name });
/** Returns the name it is kept under in Recently Deleted. */
export const deletePalette = (id: string) => invoke<string>("delete_palette", { id });
export const restorePalette = (kept: string) => invoke<Theme>("restore_palette", { kept });

/** The colours a new palette starts with: a paper, an ink and one accent. */
export function emptyDraft(name: string): PaletteDraft {
  return {
    name,
    swatches: [{ hex: "#F4F1EA" }, { hex: "#2D2A26" }, { hex: "#C2532E" }],
    roles: {},
    typography: null,
    based_on: null,
  };
}

export type ExportFormat = "json" | "csv" | "hex";

/** The swatches as text to paste elsewhere. The JSON is the shape the importer reads back. */
export function paletteText(swatches: PaletteSwatch[], format: ExportFormat): string {
  const bare = (hex: string) => hex.replace("#", "").toLowerCase();
  if (format === "csv") return swatches.map((swatch) => bare(swatch.hex)).join(",");
  if (format === "hex") return swatches.map((swatch) => swatch.hex).join(" ");
  const entries = swatches.map((swatch) => ({ name: swatch.name ?? null, hex: bare(swatch.hex) }));
  return JSON.stringify(entries, null, 2);
}
