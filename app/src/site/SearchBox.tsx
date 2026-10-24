import { useEffect, useRef, useState } from "react";
import { errorText, searchPlaces } from "../lib/ipc";
import type { Place } from "../lib/types";
import { Icon } from "../ui/Icon";

// Wait for a pause in typing before asking the geocoder.
const SEARCH_DELAY_MS = 220;
const MIN_QUERY_LENGTH = 2;

/** Address search with type-ahead. Arrow keys move through results, Return picks one. */
export function SearchBox({ onPick }: { onPick: (place: Place) => void }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Place[]>([]);
  const [highlighted, setHighlighted] = useState(0);
  const [isOpen, setOpen] = useState(false);
  const [isSearching, setSearching] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const requestId = useRef(0);

  useEffect(() => {
    const trimmed = query.trim();
    if (trimmed.length < MIN_QUERY_LENGTH) {
      setResults([]);
      setError(null);
      return;
    }
    const id = ++requestId.current;
    const timer = window.setTimeout(() => {
      setSearching(true);
      searchPlaces(trimmed)
        .then((found) => {
          if (id !== requestId.current) return;
          setResults(found);
          setHighlighted(0);
          setError(null);
        })
        .catch((reason) => {
          if (id === requestId.current) setError(errorText(reason));
        })
        .finally(() => {
          if (id === requestId.current) setSearching(false);
        });
    }, SEARCH_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [query]);

  const pick = (place: Place) => {
    setQuery(place.label);
    setOpen(false);
    onPick(place);
  };

  const showList = isOpen && query.trim().length >= MIN_QUERY_LENGTH;

  return (
    <div className="search">
      <label className="search-field">
        <span className="search-icon">
          <Icon name="search" />
        </span>
        <input
          value={query}
          placeholder="Search an address or place"
          spellCheck={false}
          onFocus={(event) => {
            setOpen(true);
            event.target.select();
          }}
          onBlur={() => setOpen(false)}
          onChange={(event) => {
            setQuery(event.target.value);
            setOpen(true);
          }}
          onKeyDown={(event) => {
            if (event.key === "ArrowDown") {
              setHighlighted((index) => Math.min(results.length - 1, index + 1));
              event.preventDefault();
            }
            if (event.key === "ArrowUp") {
              setHighlighted((index) => Math.max(0, index - 1));
              event.preventDefault();
            }
            if (event.key === "Enter" && results[highlighted]) pick(results[highlighted]);
            if (event.key === "Escape") (event.target as HTMLInputElement).blur();
          }}
        />
        {isSearching && <span className="search-spinner" aria-label="Searching" />}
      </label>
      {showList && (
        <div className="search-results">
          {error && <div className="search-empty">{error}</div>}
          {!error && results.length === 0 && !isSearching && <div className="search-empty">No places found</div>}
          {results.map((place, index) => (
            <button
              type="button"
              key={`${place.label}-${index}`}
              className={index === highlighted ? "search-result is-highlighted" : "search-result"}
              onMouseDown={(event) => {
                event.preventDefault();
                pick(place);
              }}
              onMouseEnter={() => setHighlighted(index)}
            >
              <span className="search-result-label">{place.label}</span>
              <span className="search-result-kind">{place.kind}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
