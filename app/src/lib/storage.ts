// Small per-machine preferences (export settings, tour edition, panel state).

export function readPreference<T>(key: string, fallback: T): T {
  try {
    const stored = localStorage.getItem(`openarchi.${key}`);
    return stored === null ? fallback : (JSON.parse(stored) as T);
  } catch {
    return fallback;
  }
}

export function writePreference<T>(key: string, value: T) {
  try {
    localStorage.setItem(`openarchi.${key}`, JSON.stringify(value));
  } catch {
    // Preferences are a convenience; losing one is harmless.
  }
}
