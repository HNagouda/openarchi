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
  | "contour"
  | "business"
  | "facility"
  | "transit_stop"
  | "walk_path"
  | "walk_band"
  | "road_noise"
  | "all_noise"
  | "no2"
  | "pm10"
  | "pm25"
  | "flood"
  | "slope"
  | "population";

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
  title_style: TextStyle;
  subtitle_style: TextStyle;
  title_position: Position | null;
  subtitle_position: Position | null;
  texts: TextBox[];
};

export type Align = "left" | "centre" | "right";

/** Every field left out follows the theme. Colours are "#RRGGBB" or "@role". */
export type TextStyle = {
  family?: string | null;
  bold?: boolean | null;
  italic?: boolean | null;
  size_pt?: number | null;
  align?: Align | null;
  colour?: string | null;
  opacity?: number | null;
};

/** Millimetres from the paper's top-left corner, at the top of the first line. */
export type Position = { x_mm: number; y_mm: number };

export type TextBox = { id: string; text: string; style: TextStyle; position: Position | null };

export type LayerOverride = {
  hidden?: boolean | null;
  fill?: string | null;
  stroke?: string | null;
  stroke_mm?: number | null;
  opacity?: number | null;
};

export type ViewMode = "plan" | "axonometric" | "explode";

export type View = {
  mode: ViewMode;
  rotation_deg: number;
  tilt_deg: number;
  /** Diagram ids stacked in the explode view, top plate first. */
  plates: string[];
  stacking: boolean;
};

/** A day and Dutch clock time, for the sun path, shadows and axonometric lighting. */
export type SunMoment = { month: number; day: number; hour: number; minute: number };

export type Look = {
  diagram_id: string;
  /** One of the diagram's variants; null or missing draws its first. */
  variant?: string | null;
  theme_id: string;
  sheet: Sheet;
  overrides: Partial<Record<Layer, LayerOverride>>;
  view: View;
  sun: SunMoment;
  /** The sheet's tab name; empty or missing shows "Sheet N". */
  name?: string;
  /** A smaller area inside the fetched site this sheet frames and zooms to. */
  selection?: Area | null;
  /** Leaves the project's pins off this sheet. */
  hide_pins?: boolean;
};

// crates/core/src/pin.rs
export type PinIcon = "pin" | "star" | "flag" | "dot" | "number";
/** A landmark in RD New metres, shown on every sheet that does not hide pins. */
export type Pin = { id: string; icon: PinIcon; label: string; x: number; y: number };

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
  /** The heading the diagram column lists it under. */
  group: string;
  description: string;
  /** Empty when the diagram has variants; each variant names its own. */
  layers: Layer[];
  emphasis: Layer | null;
  mute_others: boolean;
  /** A computed sun drawing on top of the layers. */
  solar?: "sun_path" | "shadow" | null;
  /** Zooms the plan out to the walk band of this many minutes. */
  walk_minutes?: number | null;
  /** Other layers or measures for the same question; the first is the default. */
  variants: Variant[];
};

export type Variant = {
  id: string;
  /** The short label on the switch: "PM10". */
  name: string;
  /** The sheet title when it is drawn: "Air quality, PM10". */
  title: string;
  description: string;
  layers: Layer[];
  emphasis: Layer | null;
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

export type Credit = { name: string; license: string; url: string; layers?: Layer[] };

export type ProjectView = {
  id: string;
  name: string;
  modified: string;
  area: Area;
  look: Look;
  /** Every sheet in tab order, the first being `look`. */
  sheets: Look[];
  pins: Pin[];
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
export type RatioUnit = "square_metres" | "buildings" | "parcels" | "places" | "stops";
export type RatioRow = { label: string; value: number; share: number; fill: string };
export type RatioGroup = { id: string; title: string; unit: RatioUnit; total: number; rows: RatioRow[]; note: string | null };
export type Ratios = {
  site_area_m2: number;
  building_count: number;
  coverage: number;
  floor_area_ratio: number;
  /** BAG floor area over site area; null when the site has no BAG units. */
  bag_floor_area_ratio: number | null;
  dwellings: number | null;
  /** CBS residents, each 100 m square counted by its share inside the site. */
  residents: number | null;
  groups: RatioGroup[];
};
export type ChartStyle = "bar" | "pie";

/** The parts of the analysis, each loaded on its own; see crates/core/src/analysis.rs. */
export type AnalysisPart = "location" | "site" | "climate" | "rain";
export type AnalysisFigure = { id: string; width_mm: number; height_mm: number; svg: string };
export type AnalysisSection = {
  id: string;
  title: string;
  facts: { label: string; value: string }[];
  figures: AnalysisFigure[];
  caption: string | null;
  notes: string[];
  source: string;
  credits: Credit[];
  warnings: string[];
};
