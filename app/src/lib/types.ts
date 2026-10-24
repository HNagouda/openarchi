// The core's types as they cross the bridge, mirroring crates/core/src.
// Field names stay snake_case because that is what serde sends.

export type LonLat = { lon: number; lat: number };

export type Area =
  | { kind: "circle"; center: LonLat; radius_m: number }
  | { kind: "rectangle"; south_west: LonLat; north_east: LonLat }
  | { kind: "polygon"; points: LonLat[] };

export type Layer =
  | "building"
  | "road"
  | "footpath"
  | "cycleway"
  | "parking"
  | "rail_area"
  | "rail_track"
  | "water"
  | "water_bank"
  | "green"
  | "verge"
  | "yard"
  | "paved"
  | "unpaved"
  | "bridge"
  | "tree"
  | "parcel"
  | "contour";

export type Paper = "a4" | "a3" | "a2" | "a1" | "a0" | "square";
export type Frame = "circle" | "square" | "area";

export type Sheet = {
  paper: Paper;
  landscape: boolean;
  scale: number;
  frame: Frame;
  title: string;
  subtitle: string;
  show_legend: boolean;
  show_north_arrow: boolean;
  show_scale_bar: boolean;
  show_credits: boolean;
};

export type LayerOverride = {
  hidden?: boolean | null;
  fill?: string | null;
  stroke?: string | null;
  stroke_mm?: number | null;
  opacity?: number | null;
};

export type Look = {
  diagram_id: string;
  theme_id: string;
  sheet: Sheet;
  overrides: Partial<Record<Layer, LayerOverride>>;
};

export type Palette = {
  paper: string;
  ground: string;
  figure: string;
  accent: string;
  water: string;
  green: string;
  ink: string;
  muted: string;
  unknown: string;
};

export type LayerStyle = {
  fill: string | null;
  stroke: string | null;
  stroke_mm: number;
  opacity: number;
};

export type Theme = {
  id: string;
  name: string;
  author: string;
  description: string;
  palette: Palette;
  ramp: string[];
  categories: string[];
  typography: { family: string; uppercase_labels: boolean };
  layers: Partial<Record<Layer, LayerStyle>>;
};

export type Diagram = {
  id: string;
  name: string;
  description: string;
  layers: Layer[];
  emphasis: Layer | null;
  mute_others: boolean;
};

export type Catalog = {
  diagrams: Diagram[];
  themes: Theme[];
  warnings: string[];
  layers: { id: Layer; label: string }[];
};

export type ProjectSummary = {
  id: string;
  name: string;
  modified: string;
  has_scene: boolean;
  look: Look | null;
};

export type Credit = { name: string; license: string; url: string };

export type ProjectView = {
  id: string;
  name: string;
  modified: string;
  area: Area;
  look: Look;
  has_scene: boolean;
  fetched_at: string | null;
  warnings: string[];
  credits: Credit[];
  layers_present: Layer[];
};

export type Place = { label: string; kind: string; location: LonLat };

export type AreaSize = { m2: number; max_m2: number; in_netherlands: boolean };

export type Progress = { step: string; done: number; total: number };

export type ExportFormat = "svg" | "pdf" | "png" | "dxf";

// crates/core/src/ratios.rs
export type RatioUnit = "square_metres" | "buildings" | "parcels";
export type RatioRow = { label: string; value: number; share: number; fill: string };
export type RatioGroup = { id: string; title: string; unit: RatioUnit; total: number; rows: RatioRow[]; note: string | null };
export type Ratios = {
  site_area_m2: number;
  building_count: number;
  coverage: number;
  floor_area_ratio: number;
  groups: RatioGroup[];
};
export type ChartStyle = "bar" | "pie";
