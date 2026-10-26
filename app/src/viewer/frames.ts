// Saving what the 3D view shows: one picture, the sun's day as a numbered
// run of pictures, or a turn round the site. A run of PNGs rather than a
// video, since every video tool reads one and the window needs no encoder.
//
// #! NOTE[3d, verify] : Does MediaRecorder on a canvas stream work in WKWebView and WebView2? If so a video could join the PNG runs

import { open, save } from "@tauri-apps/plugin-dialog";
import { saveViewPng } from "../lib/ipc";
import type { SunDay } from "../lib/types";
import type { Stage } from "./stage";
import { daylight, sunAt } from "./sunDay";

// A frame every quarter of an hour from sunrise to sunset, and every 5 degrees round the site.
const DAY_STEP_MINUTES = 15;
const ORBIT_FRAMES = 72;
const FULL_TURN_DEG = 360;
const FRAME_DIGITS = 4;
// The credit a picture carries when the aerial photo shows: small, in the bottom right corner, on a pale band.
const CREDIT_FONT_PX = 13;
const CREDIT_PADDING_PX = 6;
const CREDIT_BAND = "rgba(255, 255, 255, 0.75)";
const CREDIT_INK = "#333333";

/** Asks where to save the current picture and saves it. Null when cancelled. */
export async function savePicture(stage: Stage, stem: string, credit: string | null): Promise<string | null> {
  const path = await save({ defaultPath: `${stem}.png`, filters: [{ name: "PNG", extensions: ["png"] }] });
  if (!path) return null;
  return saveViewPng(path, capture(stage, credit));
}

/** Asks for a folder and saves the sun's day into it, one picture per step. Returns how many, or null when cancelled. */
export async function saveDay(stage: Stage, day: SunDay, stem: string, credit: string | null, onProgress: (share: number) => void): Promise<number | null> {
  const folder = await chooseFolder();
  if (!folder) return null;
  const [sunrise, sunset] = daylight(day);
  const minutes: number[] = [];
  for (let minute = sunrise; minute <= sunset; minute += DAY_STEP_MINUTES) minutes.push(minute);
  for (const [index, minute] of minutes.entries()) {
    stage.setSun(sunAt(day, minute));
    await saveViewPng(framePath(folder, `${stem}-sun`, index), capture(stage, credit));
    onProgress((index + 1) / minutes.length);
  }
  return minutes.length;
}

/** Asks for a folder and saves one turn round the site into it. Returns how many, or null when cancelled. */
export async function saveOrbit(stage: Stage, stem: string, credit: string | null, onProgress: (share: number) => void): Promise<number | null> {
  const folder = await chooseFolder();
  if (!folder) return null;
  for (let index = 0; index < ORBIT_FRAMES; index += 1) {
    await saveViewPng(framePath(folder, `${stem}-orbit`, index), capture(stage, credit));
    stage.turnCamera(FULL_TURN_DEG / ORBIT_FRAMES);
    onProgress((index + 1) / ORBIT_FRAMES);
  }
  return ORBIT_FRAMES;
}

/** "Westermarkt, Amsterdam" -> "westermarkt-amsterdam", for file names. */
export function fileStem(name: string): string {
  const stem = name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return stem || "site";
}

/** The picture as base64 PNG without the data URL prefix, with `credit` in its corner when given. */
function capture(stage: Stage, credit: string | null): string {
  const picture = stage.picture();
  if (!credit) return pngOf(picture);
  const canvas = document.createElement("canvas");
  canvas.width = picture.width;
  canvas.height = picture.height;
  const context = canvas.getContext("2d");
  if (!context) return pngOf(picture);
  context.drawImage(picture, 0, 0);
  // The canvas has device pixels; the credit is sized as it reads on screen.
  const scale = picture.width / Math.max(picture.clientWidth, 1);
  const fontPx = CREDIT_FONT_PX * scale;
  const padding = CREDIT_PADDING_PX * scale;
  context.font = `${fontPx}px system-ui, sans-serif`;
  const width = context.measureText(credit).width;
  context.fillStyle = CREDIT_BAND;
  context.fillRect(canvas.width - width - padding * 2, canvas.height - fontPx - padding * 2, width + padding * 2, fontPx + padding * 2);
  context.fillStyle = CREDIT_INK;
  context.textBaseline = "bottom";
  context.fillText(credit, canvas.width - width - padding, canvas.height - padding);
  return pngOf(canvas);
}

function pngOf(canvas: HTMLCanvasElement): string {
  return canvas.toDataURL("image/png").split(",")[1];
}

async function chooseFolder(): Promise<string | null> {
  const folder = await open({ directory: true, multiple: false });
  return typeof folder === "string" ? folder : null;
}

/** "folder/westermarkt-sun-0001.png", numbered from 1 so video tools read them in order. */
function framePath(folder: string, stem: string, index: number): string {
  return `${folder}/${stem}-${String(index + 1).padStart(FRAME_DIGITS, "0")}.png`;
}
