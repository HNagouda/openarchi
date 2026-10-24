// Colour maths for the picker (hex, RGB and HSV) and the recent colours list.

import { readPreference, writePreference } from "../lib/storage";

/** Channels 0 to 255. */
export type Rgb = { r: number; g: number; b: number };
/** Hue 0 to 360, saturation and value 0 to 1. */
export type Hsv = { h: number; s: number; v: number };

const HEX_PATTERN = /^#?([0-9a-f]{6})$/i;
const SHORT_HEX_PATTERN = /^#?([0-9a-f])([0-9a-f])([0-9a-f])$/i;
const RECENT_KEY = "recentColours";
// One row of swatches in the picker.
const MAX_RECENT = 10;

/** "3a7", "#33AA77" or "33aa77" -> "#33AA77"; anything else -> null. */
export function parseHex(text: string): string | null {
  const trimmed = text.trim();
  const full = HEX_PATTERN.exec(trimmed);
  if (full) return `#${full[1].toUpperCase()}`;
  const short = SHORT_HEX_PATTERN.exec(trimmed);
  if (short) return `#${short[1]}${short[1]}${short[2]}${short[2]}${short[3]}${short[3]}`.toUpperCase();
  return null;
}

export function hexToRgb(hex: string): Rgb {
  const value = parseInt(hex.slice(1), 16);
  return { r: (value >> 16) & 255, g: (value >> 8) & 255, b: value & 255 };
}

export function rgbToHex({ r, g, b }: Rgb): string {
  const channel = (value: number) => Math.round(Math.min(255, Math.max(0, value))).toString(16).padStart(2, "0");
  return `#${channel(r)}${channel(g)}${channel(b)}`.toUpperCase();
}

export function rgbToHsv({ r, g, b }: Rgb): Hsv {
  const red = r / 255;
  const green = g / 255;
  const blue = b / 255;
  const max = Math.max(red, green, blue);
  const spread = max - Math.min(red, green, blue);
  let hue = 0;
  if (spread > 0 && max === red) hue = ((green - blue) / spread) % 6;
  else if (spread > 0 && max === green) hue = (blue - red) / spread + 2;
  else if (spread > 0) hue = (red - green) / spread + 4;

  return { h: (hue * 60 + 360) % 360, s: max === 0 ? 0 : spread / max, v: max };
}

export function hsvToRgb({ h, s, v }: Hsv): Rgb {
  const channel = (offset: number) => {
    const k = (offset + h / 60) % 6;
    return (v - v * s * Math.max(0, Math.min(k, 4 - k, 1))) * 255;
  };
  return { r: channel(5), g: channel(3), b: channel(1) };
}

export function readRecentColours(): string[] {
  const stored = readPreference<unknown>(RECENT_KEY, []);
  if (!Array.isArray(stored)) return [];
  return stored.filter((value): value is string => typeof value === "string" && parseHex(value) !== null);
}

/** Puts `hex` first in the recent colours, saved on this machine. Returns the new list. */
export function rememberColour(hex: string): string[] {
  const recent = [hex, ...readRecentColours().filter((value) => value !== hex)].slice(0, MAX_RECENT);
  writePreference(RECENT_KEY, recent);
  return recent;
}
