// The live 3D view of the site, over the sheet: orbit it, move the sun
// through a day, and save pictures. three.js loads only when this opens
// (SheetStep imports it lazily), so the rest of the app never carries it.

import { useEffect, useRef, useState } from "react";
import { errorText, sunDay as fetchSunDay, view3d } from "../lib/ipc";
import type { SunDay, SunMoment, View } from "../lib/types";
import { Button } from "../ui/Button";
import { useToast } from "../ui/Toasts";
import { fileStem, saveDay, saveOrbit, savePicture } from "./frames";
import type { Mode } from "./navigation";
import { createStage, type Stage } from "./stage";
import { clockLabel, daylight, MINUTES_PER_DAY, sunAt } from "./sunDay";
import { PERCENT } from "../lib/units";
import { ViewButtons } from "./ViewButtons";
import "./viewer.css";

// Playing, the clock runs this many minutes a second: a summer day in about ten seconds.
const CLOCK_MINUTES_PER_SECOND = 90;
const TIME_STEP_MINUTES = 5;
const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
// Days in each month of the core's reference year, 2026, which is no leap year.
const MONTH_DAYS = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

type Props = {
  themeId: string;
  /** The sheet's date and time, where the view starts. */
  moment: SunMoment;
  /** The sheet's turn and tilt, for the view that matches it. */
  sheetView: View;
  /** The project's name, for the names of saved pictures. */
  projectName: string;
  onClose: () => void;
};

export default function Viewer3D({ themeId, moment, sheetView, projectName, onClose }: Props) {
  const stem = fileStem(projectName);
  const host = useRef<HTMLDivElement>(null);
  const stage = useRef<Stage | null>(null);
  const toast = useToast();
  const [error, setError] = useState<string | null>(null);
  const [isReady, setReady] = useState(false);
  const [date, setDate] = useState({ month: moment.month, day: moment.day });
  const [day, setDay] = useState<SunDay | null>(null);
  const [minute, setMinute] = useState(moment.hour * 60 + moment.minute);
  const [isPlaying, setPlaying] = useState(false);
  const [isOrbiting, setOrbiting] = useState(false);
  const [progress, setProgress] = useState<number | null>(null);
  const [mode, setMode] = useState<Mode>("orbit");
  const angle = useRef({ rotationDeg: sheetView.rotation_deg, tiltDeg: sheetView.tilt_deg });
  const latest = useRef({ minute, day });
  latest.current = { minute, day };

  useEffect(() => {
    let isCurrent = true;
    view3d(themeId)
      .then((view) => {
        if (!isCurrent || !host.current) return;
        stage.current = createStage(host.current, view, angle.current, setMode);
        setReady(true);
      })
      .catch((reason) => {
        if (isCurrent) setError(errorText(reason));
      });
    return () => {
      isCurrent = false;
      stage.current?.dispose();
      stage.current = null;
    };
  }, [themeId]);

  useEffect(() => {
    let isCurrent = true;
    fetchSunDay(date.month, date.day)
      .then((fetched) => {
        if (isCurrent) setDay(fetched);
      })
      .catch((reason) => {
        if (isCurrent) setError(errorText(reason));
      });
    return () => {
      isCurrent = false;
    };
  }, [date]);

  useEffect(() => {
    if (isReady && day) stage.current?.setSun(sunAt(day, minute));
  }, [isReady, day, minute]);

  useEffect(() => {
    stage.current?.setOrbiting(isOrbiting);
  }, [isOrbiting, isReady]);

  // The clock runs from sunrise to sunset and round again, until paused.
  useEffect(() => {
    if (!isPlaying || !stage.current) return;
    let isRunning = true;
    stage.current.animate((seconds) => {
      const shown = latest.current;
      if (!isRunning || !shown.day) return false;
      const [sunrise, sunset] = daylight(shown.day);
      const next = shown.minute + seconds * CLOCK_MINUTES_PER_SECOND;
      setMinute(next > sunset || next < sunrise ? sunrise : next);
      return true;
    });
    return () => {
      isRunning = false;
    };
  }, [isPlaying, isReady]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      // Esc first steps back out of walking, then closes the view.
      if (stage.current?.navigation.leave()) return;
      onClose();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  const run = async (work: (stage: Stage) => Promise<number | string | null>, saved: (result: number | string) => string) => {
    if (!stage.current) return;
    setPlaying(false);
    setOrbiting(false);
    try {
      const result = await work(stage.current);
      if (result !== null) toast(saved(result));
    } catch (reason) {
      toast(`Couldn't save. ${errorText(reason)}`, { isError: true });
    } finally {
      setProgress(null);
      if (day) stage.current?.setSun(sunAt(day, latest.current.minute));
    }
  };

  const changeMonth = (month: number) => setDate({ month, day: Math.min(date.day, MONTH_DAYS[month - 1]) });
  const changeDay = (typed: number) => setDate({ ...date, day: Math.max(1, Math.min(MONTH_DAYS[date.month - 1], typed || 1)) });

  return (
    <div className="viewer" role="dialog" aria-label="3D view">
      <header className="topbar" data-tauri-drag-region>
        <Button icon="back" tip="Back to the sheet" onClick={onClose}>
          Sheet
        </Button>
        <div className="topbar-title" data-tauri-drag-region>
          <span className="project-name">3D view</span>
        </div>
        <div className="topbar-spacer" data-tauri-drag-region />
        <Button icon="camera" tip="Save this picture as PNG" disabled={!isReady} onClick={() => run((shown) => savePicture(shown, stem), (path) => `Saved ${path}`)}>
          Save picture
        </Button>
        <Button tip="Save a picture every quarter hour from sunrise to sunset" disabled={!isReady || !day} onClick={() => day && run((shown) => saveDay(shown, day, stem, setProgress), (count) => `Saved ${count} pictures of the day`)}>
          Save day…
        </Button>
        <Button tip="Save 72 pictures turning once round the site" disabled={!isReady} onClick={() => run((shown) => saveOrbit(shown, stem, setProgress), (count) => `Saved ${count} pictures round the site`)}>
          Save orbit…
        </Button>
      </header>
      <div className="viewer-stage" ref={host} data-ready={isReady || undefined}>
        {!isReady && !error && <div className="viewer-message">Building the site…</div>}
        {error && (
          <div className="viewer-message" role="alert">
            {error}
          </div>
        )}
        {isReady && <ViewButtons navigation={stage.current?.navigation ?? null} mode={mode} />}
        {progress !== null && <div className="viewer-progress">Saving… {Math.round(progress * PERCENT)} %</div>}
      </div>
      <footer className="viewer-controls">
        <Button icon={isPlaying ? "pause" : "play"} tip={isPlaying ? "Stop the sun" : "Move the sun through the day"} isActive={isPlaying} disabled={!isReady} onClick={() => setPlaying(!isPlaying)} />
        <input
          className="slider viewer-time"
          type="range"
          aria-label="Time of day"
          min={0}
          max={MINUTES_PER_DAY}
          step={TIME_STEP_MINUTES}
          value={minute}
          onChange={(event) => setMinute(Number(event.target.value))}
          style={{ ["--fill" as string]: `${(minute / MINUTES_PER_DAY) * PERCENT}%` }}
        />
        <span className="viewer-clock tabular">{clockLabel(minute)}</span>
        <select className="text-input viewer-month" aria-label="Month" value={date.month} onChange={(event) => changeMonth(Number(event.target.value))}>
          {MONTHS.map((name, index) => (
            <option key={name} value={index + 1}>
              {name}
            </option>
          ))}
        </select>
        <input className="text-input viewer-day tabular" aria-label="Day" type="number" min={1} max={MONTH_DAYS[date.month - 1]} value={date.day} onChange={(event) => changeDay(Number(event.target.value))} />
        <span className="viewer-divider" />
        <Button icon="orbit" tip="Turn round the site; drag to turn, right-drag to slide, scroll to come closer" isActive={isOrbiting} disabled={!isReady} onClick={() => setOrbiting(!isOrbiting)}>
          Orbit
        </Button>
      </footer>
    </div>
  );
}
