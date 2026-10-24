import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "maplibre-gl/dist/maplibre-gl.css";
import "./styles/tokens.css";
import "./styles/base.css";
import "./styles/ui.css";
import "./home/home.css";
import "./site/site.css";
import "./sheet/sheet.css";
import "./tour/tour.css";
import "./settings/settings.css";
import { App } from "./App";
import { isMac } from "./lib/platform";

document.documentElement.dataset.platform = isMac ? "mac" : "other";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
