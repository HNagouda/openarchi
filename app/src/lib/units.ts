// Paper units shared by the sheet and the report editor.

/** CSS pixels per millimetre at 100 %: 96 pixels and 25.4 millimetres to the inch. */
export const MM_TO_PX = 96 / 25.4;
/** Millimetres per typographic point: 72 points to the inch. */
export const MM_PER_PT = 25.4 / 72;

/** PNG resolutions both export dialogs offer. Above 600 dpi an A3 sheet passes
 * the 150 million pixels the core renders (MAX_PNG_PIXELS in
 * crates/core/src/export/convert.rs); PDF is the format for more. */
export const MIN_DPI = 72;
export const MAX_DPI = 600;
export const DEFAULT_DPI = 300;
