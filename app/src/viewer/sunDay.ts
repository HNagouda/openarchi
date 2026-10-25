// The sun through a day as the core gives it (crates/core/src/view3d.rs),
// read at any minute, and the clock times the view shows.

import type { SunDay, SunStep } from "../lib/types";

export const MINUTES_PER_DAY = 24 * 60;
// Below this the sun counts as down, as in crates/core/src/render/lighting.rs.
const SUN_UP_DEG = 0.5;

/** The sun at `minute` of the clock day, between the two nearest steps. */
export function sunAt(day: SunDay, minute: number): SunStep {
  const steps = day.steps;
  const after = steps.findIndex((step) => step.minute >= minute);
  if (after <= 0) return after === 0 ? steps[0] : steps[steps.length - 1];
  const before = steps[after - 1];
  const next = steps[after];
  const share = (minute - before.minute) / (next.minute - before.minute);
  // Azimuth runs east to west through the south, so it never wraps past north in daylight.
  return {
    minute,
    azimuth_deg: before.azimuth_deg + share * (next.azimuth_deg - before.azimuth_deg),
    altitude_deg: before.altitude_deg + share * (next.altitude_deg - before.altitude_deg),
  };
}

/** A unit vector toward the sun: x east, y north, z up. */
export function towardSun(sun: SunStep): [number, number, number] {
  const azimuth = (sun.azimuth_deg * Math.PI) / 180;
  const altitude = (sun.altitude_deg * Math.PI) / 180;
  return [Math.sin(azimuth) * Math.cos(altitude), Math.cos(azimuth) * Math.cos(altitude), Math.sin(altitude)];
}

export function isUp(sun: SunStep): boolean {
  return sun.altitude_deg > SUN_UP_DEG;
}

/** The first and last minute the sun is up, or the whole day where it never sets or never rises. */
export function daylight(day: SunDay): [number, number] {
  const up = day.steps.filter(isUp);
  if (up.length === 0) return [0, MINUTES_PER_DAY];
  return [up[0].minute, up[up.length - 1].minute];
}

/** 790 -> "13:10". */
export function clockLabel(minute: number): string {
  const rounded = Math.round(minute);
  const hours = Math.floor(rounded / 60) % 24;
  const minutes = rounded % 60;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}
