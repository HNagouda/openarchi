//! Leaves out path points nobody could see on paper.

/// Points closer than this to the line through their neighbours are left out,
/// in hundredths of a millimetre. A quarter of the finest 0.1 mm line cannot
/// be seen on paper, and dropping them halves the larger sheets, which the
/// app has to parse and paint on every change.
const SIMPLIFY_HUNDREDTHS: f64 = 2.5;

/// `points` without those within `SIMPLIFY_HUNDREDTHS` of the line through
/// the points kept around them (Douglas-Peucker). The first and last point
/// always stay, so a closed ring stays closed.
pub fn simplified(points: &[(i64, i64)]) -> Vec<(i64, i64)> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut is_kept = vec![false; points.len()];
    is_kept[0] = true;
    is_kept[points.len() - 1] = true;
    let mut spans = vec![(0, points.len() - 1)];
    while let Some((first, last)) = spans.pop() {
        let mut farthest = (first, 0.0);
        for index in first + 1..last {
            let distance = distance_to_segment(points[index], points[first], points[last]);
            if distance > farthest.1 {
                farthest = (index, distance);
            }
        }
        let (index, distance) = farthest;
        if distance > SIMPLIFY_HUNDREDTHS {
            is_kept[index] = true;
            spans.push((first, index));
            spans.push((index, last));
        }
    }

    points
        .iter()
        .zip(is_kept)
        .filter_map(|(point, is_kept)| is_kept.then_some(*point))
        .collect()
}

/// Distance from `point` to the segment from `start` to `end`; to `start`
/// itself when the segment has no length, as for a closed ring.
fn distance_to_segment(point: (i64, i64), start: (i64, i64), end: (i64, i64)) -> f64 {
    let (px, py) = (point.0 as f64, point.1 as f64);
    let (sx, sy) = (start.0 as f64, start.1 as f64);
    let (dx, dy) = (end.0 as f64 - sx, end.1 as f64 - sy);
    let length_squared = dx * dx + dy * dy;
    if length_squared == 0.0 {
        return (px - sx).hypot(py - sy);
    }
    let along = (((px - sx) * dx + (py - sy) * dy) / length_squared).clamp(0.0, 1.0);

    (px - sx - along * dx).hypot(py - sy - along * dy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_a_hundredth_off_a_straight_edge_are_left_out() {
        let edge = [(0, 0), (100, 1), (200, 0), (300, -1), (400, 0)];

        assert_eq!(simplified(&edge), vec![(0, 0), (400, 0)]);
    }

    #[test]
    fn corners_stay() {
        let corner = [(0, 0), (100, 0), (100, 100)];

        assert_eq!(simplified(&corner), corner.to_vec());
    }

    #[test]
    fn a_closed_ring_keeps_its_far_side() {
        let ring = [(0, 0), (100, 0), (100, 100), (0, 100), (0, 0)];

        assert_eq!(simplified(&ring), ring.to_vec());
    }
}
