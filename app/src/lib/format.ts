// How numbers and dates read in the UI.

const HECTARE_M2 = 10_000;
const SQUARE_KM_M2 = 1_000_000;

const wholeNumber = new Intl.NumberFormat("en-GB", { maximumFractionDigits: 0 });
const oneDecimal = new Intl.NumberFormat("en-GB", { maximumFractionDigits: 1, minimumFractionDigits: 1 });
const twoDecimals = new Intl.NumberFormat("en-GB", { maximumFractionDigits: 2, minimumFractionDigits: 2 });

/** 2430 -> "2,430 m²", 52_000 -> "5.2 ha", 3_100_000 -> "3.10 km²". */
export function formatArea(m2: number): string {
  if (m2 < HECTARE_M2) return `${wholeNumber.format(m2)} m²`;
  if (m2 < SQUARE_KM_M2) return `${oneDecimal.format(m2 / HECTARE_M2)} ha`;
  return `${twoDecimals.format(m2 / SQUARE_KM_M2)} km²`;
}

/** 312.4 -> "312 m", 1520 -> "1.52 km". */
export function formatDistance(metres: number): string {
  if (metres < 1000) return `${wholeNumber.format(metres)} m`;
  return `${twoDecimals.format(metres / 1000)} km`;
}

export function formatScale(denominator: number): string {
  return `1:${wholeNumber.format(denominator).replace(/,/g, " ")}`;
}

/** A typed number that may use a comma as decimal mark: "0,5" -> 0.5. */
export function parseTyped(text: string): number | null {
  const cleaned = text.trim().replace(/\s| /g, "").replace(",", ".").replace(/^1:/, "");
  if (cleaned === "") return null;
  const value = Number(cleaned);
  return Number.isFinite(value) ? value : null;
}

/** `value` kept between `min` and `max`; `min` wins when the range is empty, so a block wider than its page sits at its edge. */
export function clamp(value: number, min: number, max: number): number {
  return Math.min(Math.max(value, min), Math.max(min, max));
}

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;

/** "Just now", "12 min ago", "Yesterday", "3 Oct", "3 Oct 2025". */
export function formatModified(rfc3339: string, now = new Date()): string {
  const date = new Date(rfc3339);
  if (Number.isNaN(date.getTime())) return "";
  const elapsed = now.getTime() - date.getTime();
  if (elapsed < MINUTE_MS) return "Just now";
  if (elapsed < HOUR_MS) return `${Math.floor(elapsed / MINUTE_MS)} min ago`;
  if (elapsed < DAY_MS && date.getDate() === now.getDate()) {
    return `Today, ${date.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" })}`;
  }
  if (elapsed < 2 * DAY_MS) return "Yesterday";
  const sameYear = date.getFullYear() === now.getFullYear();
  return date.toLocaleDateString("en-GB", { day: "numeric", month: "short", year: sameYear ? undefined : "numeric" });
}
