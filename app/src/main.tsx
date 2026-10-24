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

function start() {
  createRoot(document.getElementById("root")!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
}

// In a plain browser (for screenshots) the Rust side is mocked. The check on
// import.meta.env.DEV is constant in a release build, so the mock is left out of it.
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) import("./dev/mockIpc").then(start);
else start();
