// Telling a mouse wheel from a trackpad swipe, which arrive as the same event.
//
// A mouse wheel turns in notches: no sideways part, and WebKit and Chromium
// still report its old `wheelDeltaY` as a multiple of 120. A trackpad sends
// smooth pixel amounts, often with a sideways part. A pinch arrives as a wheel
// with ctrlKey in Chromium (WebView2) and as gesture events in WebKit.
//
// #! NOTE[3d, verify] : The notch test is a heuristic; check it with a real mouse and trackpad in WKWebView and WebView2

// Pixels per line, for wheels that report lines (Firefox, some Windows mice).
const LINE_PX = 16;
const NOTCH = 120;

export function isTrackpadScroll(event: WheelEvent): boolean {
  if (event.deltaMode !== WheelEvent.DOM_DELTA_PIXEL) return false;
  if (event.deltaX !== 0) return true;
  const notches = (event as WheelEvent & { wheelDeltaY?: number }).wheelDeltaY;
  return notches !== undefined && notches % NOTCH !== 0;
}

/** The wheel's movement in pixels, sideways and down. */
export function wheelPixels(event: WheelEvent): [number, number] {
  const scale = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? LINE_PX : 1;
  return [event.deltaX * scale, event.deltaY * scale];
}
