//! The last few grids worked out from a scene, so a sheet redrawn for a new
//! title, palette or thumbnail does not work them out again, and the last few
//! analysis parts, so a report's blocks do not each draw their part again.
//!
//! Hours of sun, sky view, solar radiation, city blocks and the wind flow are
//! worked out each time a sheet draws them, never stored, and the app redraws
//! the sheet on every edit. They take from a few to several hundred
//! milliseconds, far longer than drawing the rest of a sheet. Each is kept
//! with what it was worked out from, compared in full rather than hashed, so
//! any change to those inputs works it out afresh. What a grid is kept as is
//! the grid before banding, so new class breaks only band it again.

// #! NOTE[cache] : Module-level state on purpose. The app's state could only hand a cache to the drawing through a new RenderInput field, built at 63 places, and the MCP server loads the project afresh for every call, so a cache it owned would never be hit. These few Mutexes serve both.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use geo::Polygon;

use crate::analysis::{Part, Section};
use crate::area::Area;
use crate::blocks;
use crate::radiation::{RadiationGrid, SolarYear};
use crate::scene::{Feature, Layer, Scene};
use crate::skyview::SkyGrid;
use crate::sources::raster::Grid;
use crate::sunlight::SunGrid;
use crate::theme::Theme;
use crate::wind::{FlowKey, WindFlow};

/// Enough for every sheet of one site, or of a few sites at once.
const GRIDS_KEPT: usize = 6;
/// Three parts in a couple of palettes, for a couple of sites.
const PARTS_KEPT: usize = 12;

pub static SUN_HOURS: Memo<(SceneKey, u32, u32), SunGrid> = Memo::new(GRIDS_KEPT);
pub static SKY_VIEWS: Memo<SceneKey, SkyGrid> = Memo::new(GRIDS_KEPT);
pub static RADIATION: Memo<SceneKey, RadiationGrid> = Memo::new(GRIDS_KEPT);
pub static BLOCKS: Memo<SceneKey, Vec<Feature>> = Memo::new(GRIDS_KEPT);
pub static WIND_FLOWS: Memo<FlowKey, WindFlow> = Memo::new(GRIDS_KEPT);
/// Location, climate and rain of a few sites, each in a few palettes.
pub static ANALYSIS_PARTS: Memo<PartKey, Vec<Section>> = Memo::new(PARTS_KEPT);

/// A few values, each kept with the key it was worked out from, oldest first.
pub struct Memo<K, V> {
    kept: Mutex<Vec<(K, Arc<V>)>>,
    size: usize,
}

impl<K: PartialEq, V> Memo<K, V> {
    pub const fn new(size: usize) -> Memo<K, V> {
        Memo {
            kept: Mutex::new(Vec::new()),
            size,
        }
    }

    /// The kept value for `key`, or the one `work` makes, kept for next time.
    /// The lock is held while working, so two drawings of one sheet at once work it out once.
    pub fn kept_or(&self, key: K, work: impl FnOnce() -> V) -> Arc<V> {
        let mut kept = self.lock();
        if let Some((_, value)) = kept.iter().find(|(kept_key, _)| *kept_key == key) {
            return value.clone();
        }

        let value = Arc::new(work());
        kept.push((key, value.clone()));
        if kept.len() > self.size {
            kept.remove(0);
        }
        value
    }

    /// Like `kept_or`, for work that can fail. A failure is not kept, so
    /// the next call tries again, after a lost connection say.
    pub fn kept_or_try<E>(&self, key: K, work: impl FnOnce() -> Result<V, E>) -> Result<Arc<V>, E> {
        let mut kept = self.lock();
        if let Some((_, value)) = kept.iter().find(|(kept_key, _)| *kept_key == key) {
            return Ok(value.clone());
        }

        let value = Arc::new(work()?);
        kept.push((key, value.clone()));
        if kept.len() > self.size {
            kept.remove(0);
        }
        Ok(value)
    }

    /// A poisoned lock only means an earlier drawing panicked; the kept values are still whole.
    fn lock(&self) -> MutexGuard<'_, Vec<(K, Arc<V>)>> {
        self.kept
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Everything in a scene that the hours of sun, the sky view, the solar
/// radiation and the city blocks are worked out from. Each reads only part of
/// it; one key for all is simpler, and the rest never changes without a new fetch.
pub struct SceneKey {
    area: Area,
    boundary: Polygon<f64>,
    features: Vec<Feature>,
    context: Vec<Feature>,
    terrain: Option<Grid>,
    canopy: Option<Grid>,
    solar: Option<SolarYear>,
}

impl SceneKey {
    pub fn of(scene: &Scene) -> SceneKey {
        let is_read = |feature: &&Feature| {
            matches!(feature.layer, Layer::Building | Layer::Tree)
                || blocks::STREET_LAYERS.contains(&feature.layer)
        };
        SceneKey {
            area: scene.area.clone(),
            boundary: scene.boundary.clone(),
            features: scene.features.iter().filter(is_read).cloned().collect(),
            context: scene.context.iter().filter(is_read).cloned().collect(),
            terrain: scene.terrain.clone(),
            canopy: scene.measured.grids.get(&Layer::TreeCanopy).cloned(),
            solar: scene.solar.clone(),
        }
    }
}

impl PartialEq for SceneKey {
    fn eq(&self, other: &SceneKey) -> bool {
        self.area == other.area
            && self.boundary == other.boundary
            && self.features == other.features
            && self.context == other.context
            && is_same_grid(self.terrain.as_ref(), other.terrain.as_ref())
            && is_same_grid(self.canopy.as_ref(), other.canopy.as_ref())
            && self.solar == other.solar
    }
}

/// What an analysis part other than the site is drawn from: the place, the
/// palette, the cache its data comes from, and for the rain the last full year.
#[derive(PartialEq)]
pub struct PartKey {
    pub part: Part,
    pub origin: (f64, f64),
    pub boundary: Polygon<f64>,
    pub theme: Theme,
    pub cache: PathBuf,
    pub last_year: i32,
}

/// Grids compared bit for bit, because their missing cells are NaN, which
/// never equals itself.
fn is_same_grid(a: Option<&Grid>, b: Option<&Grid>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.west == b.west
                && a.north == b.north
                && a.cell_m == b.cell_m
                && a.width == b.width
                && a.height == b.height
                && a.values.len() == b.values.len()
                && a.values
                    .iter()
                    .zip(&b.values)
                    .all(|(a, b)| a.to_bits() == b.to_bits())
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forest::tests::{canopy, wooded};
    use crate::grid::tests::{building, site, square};

    fn scene() -> Scene {
        wooded(
            site(
                vec![building(square(0.0, 0.0, 10.0, 10.0), 9.0)],
                Vec::new(),
            ),
            canopy(&[(20.0, 20.0, 120.0, 120.0)], 15.0),
        )
    }

    #[test]
    fn the_same_scene_gives_the_same_key_despite_missing_cells() {
        assert!(SceneKey::of(&scene()) == SceneKey::of(&scene()));
    }

    #[test]
    fn a_change_to_any_input_gives_another_key() {
        let mut taller = scene();
        taller.features[0].attributes.height_m = Some(12.0);
        let mut moved = scene();
        moved.boundary = square(-50.0, -50.0, 50.0, 50.0);
        let mut without_trees = scene();
        without_trees.measured.grids.clear();
        let mut on_terrain = scene();
        on_terrain.terrain = Some(canopy(&[], 0.0));

        for changed in [taller, moved, without_trees, on_terrain] {
            assert!(SceneKey::of(&scene()) != SceneKey::of(&changed));
        }
    }

    #[test]
    fn a_kept_value_is_not_worked_out_again() {
        let memo: Memo<u32, u32> = Memo::new(2);
        assert_eq!(*memo.kept_or(1, || 10), 10);
        assert_eq!(*memo.kept_or(1, || unreachable!()), 10);
        assert_eq!(*memo.kept_or(2, || 20), 20);
        assert_eq!(*memo.kept_or(3, || 30), 30);
        // The oldest made room.
        assert_eq!(*memo.kept_or(1, || 11), 11);
    }
}
