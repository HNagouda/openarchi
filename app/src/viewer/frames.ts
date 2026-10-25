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

/** Asks where to save the current picture and saves it. Null when cancelled. */
export async function savePicture(stage: Stage, stem: string): Promise<string | null> {
  const path = await save({ defaultPath: `${stem}.png`, filters: [{ name: "PNG", extensions: ["png"] }] });
  if (!path) return null;
  return saveViewPng(path, stage.capture());
}

/** Asks for a folder and saves the sun's day into it, one picture per step. Returns how many, or null when cancelled. */
export async function saveDay(stage: Stage, day: SunDay, stem: string, onProgress: (share: number) => void): Promise<number | null> {
  const folder = await chooseFolder();
  if (!folder) return null;
  const [sunrise, sunset] = daylight(day);
  const minutes: number[] = [];
  for (let minute = sunrise; minute <= sunset; minute += DAY_STEP_MINUTES) minutes.push(minute);
  for (const [index, minute] of minutes.entries()) {
    stage.setSun(sunAt(day, minute));
    await saveViewPng(framePath(folder, `${stem}-sun`, index), stage.capture());
    onProgress((index + 1) / minutes.length);
  }
  return minutes.length;
}

/** Asks for a folder and saves one turn round the site into it. Returns how many, or null when cancelled. */
export async function saveOrbit(stage: Stage, stem: string, onProgress: (share: number) => void): Promise<number | null> {
  const folder = await chooseFolder();
  if (!folder) return null;
  for (let index = 0; index < ORBIT_FRAMES; index += 1) {
    await saveViewPng(framePath(folder, `${stem}-orbit`, index), stage.capture());
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

async function chooseFolder(): Promise<string | null> {
  const folder = await open({ directory: true, multiple: false });
  return typeof folder === "string" ? folder : null;
}

/** "folder/westermarkt-sun-0001.png", numbered from 1 so video tools read them in order. */
function framePath(folder: string, stem: string, index: number): string {
  return `${folder}/${stem}-${String(index + 1).padStart(FRAME_DIGITS, "0")}.png`;
}
