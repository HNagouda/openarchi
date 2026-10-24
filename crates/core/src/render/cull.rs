//! Leaves out features that lie wholly outside the frame.
//!
//! A sheet zoomed in on part of a large site would otherwise carry every
//! building of the site, clipped away but still written, sent to the app,
//! parsed and painted on every change. Buildings and trees keep a margin,
//! since their shadows and their tilted walls reach into the frame from
//! outside it.

use std::collections::BTreeMap;

use geo::{BoundingRect, Rect};

use super::geometry::Projection;
use super::shadow;
use crate::scene::{Feature, Layer};
use crate::sun::SunPosition;

/// Strokes, point symbols and labels reach a little past a feature's own
/// outline, in ground metres.
const SLACK_M: f64 = 5.0;

/// `features` without those whose bounding box misses `bounds`, the frame on
/// the ground. Casters (buildings and trees) count as `caster_margin_m`
/// larger. With no bounds, everything is kept.
pub fn within_frame(
    features: BTreeMap<Layer, Vec<&Feature>>,
    bounds: Option<Rect<f64>>,
    caster_margin_m: f64,
) -> BTreeMap<Layer, Vec<&Feature>> {
    let Some(bounds) = bounds else {
        return features;
    };

    features
        .into_iter()
        .map(|(layer, layer_features)| {
            let is_caster = matches!(layer, Layer::Building | Layer::Tree);
            let margin = if is_caster {
                SLACK_M + caster_margin_m
            } else {
                SLACK_M
            };
            let reach = grown(bounds, margin);
            let kept = layer_features
                .into_iter()
                .filter(|feature| overlaps(feature, &reach))
                .collect();
            (layer, kept)
        })
        .collect()
}

/// How far outside the frame a building or tree `tallest_m` high may stand
/// and still reach into it, in ground metres: by its shadow when the sheet
/// casts them, or by its walls rising up the paper in a tilted view.
pub fn caster_margin(
    tallest_m: f64,
    casts_shadows: bool,
    sun: &SunPosition,
    projection: &Projection,
) -> f64 {
    let shadow_per_metre = match shadow::shadow_per_metre(sun) {
        Some((east, north)) if casts_shadows => east.hypot(north),
        _ => 0.0,
    };
    let rise_per_metre = if projection.squash > 0.0 {
        projection.rise / projection.squash
    } else {
        0.0
    };

    tallest_m * (shadow_per_metre + rise_per_metre)
}

fn grown(bounds: Rect<f64>, margin: f64) -> Rect<f64> {
    let (min, max) = (bounds.min(), bounds.max());
    Rect::new(
        (min.x - margin, min.y - margin),
        (max.x + margin, max.y + margin),
    )
}

fn overlaps(feature: &Feature, reach: &Rect<f64>) -> bool {
    match feature.geometry.bounding_rect() {
        Some(own) => {
            own.min().x <= reach.max().x
                && own.max().x >= reach.min().x
                && own.min().y <= reach.max().y
                && own.max().y >= reach.min().y
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use geo::{Geometry, Point};

    use super::*;
    use crate::scene::Attributes;

    fn tree_at(x: f64, y: f64) -> Feature {
        Feature {
            layer: Layer::Tree,
            geometry: Geometry::Point(Point::new(x, y)),
            attributes: Attributes::default(),
        }
    }

    fn frame() -> Option<Rect<f64>> {
        Some(Rect::new((-100.0, -100.0), (100.0, 100.0)))
    }

    #[test]
    fn a_feature_far_outside_the_frame_is_left_out() {
        let near = tree_at(50.0, 0.0);
        let far = tree_at(500.0, 0.0);
        let features = BTreeMap::from([(Layer::Tree, vec![&near, &far])]);

        let kept = within_frame(features, frame(), 0.0);

        assert_eq!(kept[&Layer::Tree].len(), 1);
    }

    #[test]
    fn a_caster_within_its_shadow_of_the_frame_stays() {
        let outside = tree_at(150.0, 0.0);
        let features = BTreeMap::from([(Layer::Tree, vec![&outside])]);

        let kept = within_frame(features, frame(), 60.0);

        assert_eq!(kept[&Layer::Tree].len(), 1);
    }

    #[test]
    fn without_a_frame_everything_stays() {
        let far = tree_at(5000.0, 0.0);
        let features = BTreeMap::from([(Layer::Tree, vec![&far])]);

        let kept = within_frame(features, None, 0.0);

        assert_eq!(kept[&Layer::Tree].len(), 1);
    }
}
