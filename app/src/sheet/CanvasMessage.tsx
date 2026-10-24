// What the canvas shows instead of the sheet: a site not fetched yet, a
// catalog that did not load, a sheet that could not be drawn, or the wait.

import { Button } from "../ui/Button";
import { Problem } from "../ui/Problem";

export function CanvasMessage({
  catalogError,
  previewError,
  onRetry,
  hasScene,
}: {
  catalogError: string | null;
  previewError: string | null;
  onRetry: () => void;
  hasScene: boolean;
}) {
  if (!hasScene) {
    return (
      <div className="canvas-message">
        <Problem title="This site hasn't been fetched yet." detail="Use Edit site to draw it and fetch the data." />
      </div>
    );
  }
  if (catalogError) {
    return (
      <div className="canvas-message">
        <Problem title="Diagrams and palettes aren't available." detail={catalogError}>
          <Button variant="secondary" onClick={onRetry}>
            Try again
          </Button>
        </Problem>
      </div>
    );
  }
  if (previewError) {
    return (
      <div className="canvas-message">
        <Problem title="The sheet couldn't be drawn." detail={previewError} />
      </div>
    );
  }
  return <div className="canvas-message"><div className="canvas-loading" aria-label="Rendering" /></div>;
}
