// The ratios and analysis sections the library lists, fetched in the report's
// palette. Each part arrives on its own, so the quick ones show while climate
// and rain still wait on their services.

import { useEffect, useState } from "react";
import { errorText, getAnalysis, getRatios } from "../lib/ipc";
import type { AnalysisPart, AnalysisSection, Ratios } from "../lib/types";

// Every analysis part, in report order, in one place.
const PARTS: AnalysisPart[] = ["location", "site", "climate", "rain"];

export type LibraryData = {
  ratios: Ratios | null;
  sections: AnalysisSection[];
  isLoading: boolean;
  /** Parts that could not be fetched, as sentences. */
  problems: string[];
};

export function useLibraryData(themeId: string, isEnabled: boolean): LibraryData {
  const [ratios, setRatios] = useState<Ratios | null>(null);
  const [parts, setParts] = useState<Partial<Record<AnalysisPart, AnalysisSection[]>>>({});
  const [pending, setPending] = useState(0);
  const [problems, setProblems] = useState<string[]>([]);

  useEffect(() => {
    if (!isEnabled) return;
    let isCancelled = false;
    setProblems([]);
    setPending(PARTS.length + 1);
    const settle = () => {
      if (!isCancelled) setPending((count) => count - 1);
    };
    const complain = (what: string, reason: unknown) => {
      if (!isCancelled) setProblems((current) => [...current, `${what}: ${errorText(reason)}`]);
    };

    getRatios(themeId, null)
      .then((found) => !isCancelled && setRatios(found))
      .catch((reason) => complain("Ratios", reason))
      .finally(settle);
    for (const part of PARTS) {
      getAnalysis(part, themeId)
        .then((found) => !isCancelled && setParts((current) => ({ ...current, [part]: found })))
        .catch((reason) => complain(part[0].toUpperCase() + part.slice(1), reason))
        .finally(settle);
    }

    return () => {
      isCancelled = true;
    };
  }, [themeId, isEnabled]);

  const sections = PARTS.flatMap((part) => parts[part] ?? []);
  return { ratios, sections, isLoading: pending > 0, problems };
}
