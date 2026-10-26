// Dev only: the imported model and the 3D view in a plain browser. Importing any file gives
// the test L-shaped block app_fixtures placed north of the Westerkerk
// (dev-fixtures/model.json), and the preview gets its footprint drawn in, so
// moving, turning and replacing can be tried. The fixtures are rendered
// without the model, so a replaced building still shows; the real drawing
// comes from crates/core/src/render/proposal.rs.

import type { CoveredBuilding, Look, ModelChoice, ModelView } from "../lib/types";
import { rdToPaper } from "./mockPins";

const ACCENT = "#c4502a";
const OUTLINE_MM = 0.18;
const MOCK_PATH = "/Users/you/Models/house.obj";

type Fixture = ModelView & { covered: CoveredBuilding[] };

const fixture: Fixture = await fetch("/dev-fixtures/model.json").then((response) => response.json());
const models: Record<string, ModelView> = {};
let current: ModelChoice | null = null;

/** The model in effect as the project view sends it, or null. */
export function mockModelView(): ModelView | null {
  if (!current) return null;
  return { ...models[current.key], placement: current.placement, replaced: current.replaced };
}

/** The mock's answer to a model command, or undefined for any other command. */
export function answerModel(command: string, args: Record<string, unknown>): unknown {
  if (command === "plugin:dialog|open") return MOCK_PATH;
  if (command === "import_model" || command === "reread_model") {
    const key = `model-${Object.keys(models).length + 1}`;
    const { covered: _covered, ...view } = fixture;
    const placement = (args.placement as ModelView["placement"] | null) ?? view.placement;
    models[key] = { ...view, key, placement, unit: (args.unit as ModelView["unit"]) ?? view.unit, up: (args.up as ModelView["up"]) ?? view.up };
    return models[key];
  }
  if (command === "set_model") {
    current = (args.choice as ModelChoice | null) ?? null;
    return null;
  }
  if (command === "covered_buildings") return fixture.covered;
  // The 3D view: the sheet's palette, its diagram where the fixtures have it, one day; saved pictures go nowhere.
  if (command === "view_3d") return mockView3d(args.look as Look);
  if (command === "sun_day") return fetch("/dev-fixtures/sun-day.json").then((response) => response.json());
  if (command === "save_view_png") return args.path;
  return undefined;
}

// Every diagram the fixtures write a 3D view for (crates/core/examples/app_fixtures/view3d.rs).
const VIEW3D_DIAGRAMS = ["context", "building-age", "building-use"];

function mockView3d(look: Look): Promise<unknown> {
  const diagram = VIEW3D_DIAGRAMS.includes(look.diagram_id) ? look.diagram_id : "context";
  return fetch(`/dev-fixtures/view3d/${diagram}-${look.theme_id}.json`).then((response) => response.json());
}

/** The model's footprint drawn into a fixture preview where the mock's choice stands it. */
export function withMockModel(svg: string): string {
  const view = mockModelView();
  const toPaper = rdToPaper(svg);
  if (!view || !toPaper) return svg;
  const angle = (view.placement.rotation_deg * Math.PI) / 180;
  const data = view.footprint
    .map((ring) => {
      const points = ring.map(([x, y]) => {
        const rd = [view.placement.x + x * Math.cos(angle) + y * Math.sin(angle), view.placement.y - x * Math.sin(angle) + y * Math.cos(angle)];
        return toPaper(rd[0], rd[1]).join(",");
      });
      return `M${points.join("L")}Z`;
    })
    .join("");
  const path = `<g id="proposal"><path d="${data}" fill="${ACCENT}" stroke="#2b2b2b" stroke-width="${OUTLINE_MM}"/></g>`;
  return svg.replace("</svg>", `${path}</svg>`);
}
