//! The last few flows worked out, so a sheet redrawn for a new title, palette
//! or thumbnail does not solve the same flow again.
//!
//! Solving takes about half a second, far longer than drawing the rest of a
//! sheet, and the app redraws on every edit. A flow depends only on the site
//! boundary, the buildings on the grid and the wind direction, so those are
//! the key, compared in full rather than hashed.

// #! NOTE[wind, cache] : Module-level state on purpose: the flow is drawn from 60-odd RenderInput call sites, and passing a cache through all of them costs more to read than this one Mutex.

use std::sync::{Arc, Mutex, MutexGuard};

use geo::Polygon;

use super::WindFlow;
use crate::sunlight::raster::Raster;

/// Enough for each season of one site, or the sheets of a few sites.
const FLOWS_KEPT: usize = 6;

static KEPT: Mutex<Vec<Kept>> = Mutex::new(Vec::new());

/// What a flow is solved from.
#[derive(PartialEq)]
pub struct Key {
    pub from_deg: f64,
    pub boundary: Polygon<f64>,
    pub raster: Raster,
    pub solid: Vec<bool>,
}

struct Kept {
    key: Key,
    flow: Arc<WindFlow>,
}

/// The kept flow for `key`, or the one `solve` makes, kept for next time.
/// The lock is held while solving, so two drawings of one sheet at once solve it once.
pub fn kept_or(key: Key, solve: impl FnOnce() -> WindFlow) -> Arc<WindFlow> {
    let mut kept = lock();
    if let Some(found) = kept.iter().find(|kept| kept.key == key) {
        return found.flow.clone();
    }

    let flow = Arc::new(solve());
    kept.push(Kept {
        key,
        flow: flow.clone(),
    });
    if kept.len() > FLOWS_KEPT {
        kept.remove(0);
    }
    flow
}

/// A poisoned lock only means an earlier drawing panicked; the kept flows are still whole.
fn lock() -> MutexGuard<'static, Vec<Kept>> {
    KEPT.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}
