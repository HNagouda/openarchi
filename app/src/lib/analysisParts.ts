import type { AnalysisPart } from "./types";

/** Every analysis part, in report order, in one place: with its title and what the inspector says while it loads. */
export const ANALYSIS_PARTS: { part: AnalysisPart; title: string; waiting: string }[] = [
  { part: "location", title: "Location", waiting: "Looking up the address and municipality." },
  { part: "site", title: "Site", waiting: "Summing up the site." },
  { part: "climate", title: "Climate", waiting: "Fetching a typical year of weather from PVGIS. The first time takes a few seconds." },
  { part: "rain", title: "Rain", waiting: "Fetching thirty years of rain from KNMI." },
];
