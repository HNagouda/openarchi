//! Which edition of a dataset to fetch: the newest one its service offers,
//! asked when a site is fetched instead of pinned in code.
//!
//! Every service publishes a new year on its own schedule, and so will the
//! services of countries added later. How to ask is a quirk of each service
//! (a WCS capabilities list, a probe of an OGC API), so it lives in the source
//! module. This module only keeps the answer in the tile cache for a day and
//! falls back when the service cannot be asked: first to the last answer it
//! gave, then to the edition the source module names as known to work.

use std::time::{Duration, SystemTime};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::sources::cache::TileCache;
use crate::sources::grid::Tile;
use crate::Error;

/// How long an answer is trusted before the service is asked again. Services
/// publish a new edition a few times a year at most, and the largest question,
/// RIVM's capabilities, takes about a second, so asking once a day keeps it off
/// nearly every fetch while a new edition still shows up the next day.
pub const MAX_AGE: Duration = Duration::from_secs(24 * 60 * 60);

/// Longest wait for an answer. The slowest service answers in about a second;
/// past this it is struggling, and the last known edition is the better choice
/// than holding up the whole site.
pub const ASK_TIMEOUT: Duration = Duration::from_secs(5);

/// An edition is the same for every tile, so it is kept under one fixed key.
const CACHE_KEY: Tile = Tile { column: 0, row: 0 };

/// One edition of a dataset: the year its data describes, which the sheet
/// quotes, and the name its service knows it by.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Edition {
    pub year: i32,
    /// "alo__rivm_20250101_Geluid_lden_wegverkeer_2022", or "2024" for a year filter.
    pub id: String,
}

/// The answer to `ask`, kept in `cache` under `name` for `MAX_AGE`. When the
/// service cannot be asked, the last answer it gave or else `known`, with the
/// reason, so the caller can warn that its edition may not be the newest.
pub fn resolve<T: Serialize + DeserializeOwned>(
    cache: &TileCache,
    name: &str,
    ask: impl FnOnce() -> crate::Result<T>,
    known: T,
) -> (T, Option<Error>) {
    let dataset = format!("edition-{name}");
    if let Some(answer) = cached(cache, &dataset, MAX_AGE) {
        return (answer, None);
    }

    match ask() {
        Ok(answer) => {
            // A failed write only means asking again next time, which is the
            // whole cost, so the error goes no further.
            if let Ok(bytes) = serde_json::to_vec(&answer) {
                let _ = cache.write(&dataset, CACHE_KEY, &bytes);
            }
            (answer, None)
        }
        Err(error) => {
            let fallback = cached(cache, &dataset, Duration::MAX).unwrap_or(known);
            (fallback, Some(error))
        }
    }
}

/// The current calendar year in UTC, from the clock.
pub fn this_year() -> Option<i32> {
    let now = humantime::format_rfc3339_seconds(SystemTime::now()).to_string();
    now.get(0..4)?.parse().ok()
}

// A cached answer that cannot be read is asked again, which also replaces it.
fn cached<T: DeserializeOwned>(cache: &TileCache, dataset: &str, max_age: Duration) -> Option<T> {
    let bytes = cache.read_within(dataset, CACHE_KEY, max_age).ok()??;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use tempfile::TempDir;

    const NAME: &str = "test-source";

    fn unreachable() -> crate::Result<i32> {
        Err(Error::Network {
            service: "test".to_string(),
            detail: "offline".to_string(),
        })
    }

    #[test]
    fn an_answer_is_asked_once_and_then_read_from_the_cache() {
        let folder = TempDir::new().unwrap();
        let cache = TileCache::at(folder.path());
        let asked = Cell::new(0);
        let ask = || {
            asked.set(asked.get() + 1);
            Ok(2025)
        };

        let first = resolve(&cache, NAME, ask, 2024);
        let second = resolve(&cache, NAME, ask, 2024);

        assert_eq!(first.0, 2025);
        assert_eq!(second.0, 2025);
        assert!(second.1.is_none());
        assert_eq!(asked.get(), 1);
    }

    #[test]
    fn an_unreachable_service_falls_back_to_the_known_edition_with_the_reason() {
        let folder = TempDir::new().unwrap();
        let cache = TileCache::at(folder.path());

        let (year, error) = resolve(&cache, NAME, unreachable, 2024);

        assert_eq!(year, 2024);
        assert!(error.unwrap().to_string().contains("offline"));
    }

    #[test]
    fn an_unreachable_service_falls_back_to_its_last_answer_before_the_known_edition() {
        let folder = TempDir::new().unwrap();
        let cache = TileCache::at(folder.path());
        resolve(&cache, NAME, || Ok(2025), 2024);
        let path = folder.path().join("tiles-v1/edition-test-source/0_0.zst");
        let file = std::fs::File::options().write(true).open(path).unwrap();
        file.set_modified(SystemTime::now() - MAX_AGE * 2).unwrap();

        let (year, error) = resolve(&cache, NAME, unreachable, 2024);

        assert_eq!(year, 2025);
        assert!(error.is_some());
    }
}
