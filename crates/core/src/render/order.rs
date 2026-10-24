//! Back-to-front drawing order for buildings and trees in a tilted view.
//!
//! The viewer looks along `v`, so one solid can only hide another when their
//! footprints share some `u` across the paper. For every such pair whose
//! drawings also overlap up the paper, the footprints are cut by a line of
//! constant `u` and the one met first is in front. Those pairs form a graph
//! that is drawn in topological order, with the furthest solid first whenever
//! there is a choice. Sorting by a single depth value instead gets long,
//! diagonal buildings wrong next to small ones, which is common in old towns.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Width of the strips across the paper that pairs are looked for in, in ground metres.
const STRIP_M: f64 = 25.0;
/// Where along the shared `u` range the footprints are cut, tried in this order.
const CUT_SHARES: [f64; 5] = [0.5, 0.25, 0.75, 0.05, 0.95];
/// Footprints that only touch along an edge, in metres, do not count as sharing a line of sight.
const TOUCH_M: f64 = 1e-6;
/// Depth keys are compared as whole millimetres, so the heap needs no float ordering.
const KEY_PER_M: f64 = 1000.0;

/// A solid as the ordering sees it: its outline in the view frame and its height.
#[derive(Debug, Clone)]
pub struct Footprint {
    /// (u, v) in metres, as `Projection::to_view` gives them.
    pub ring: Vec<(f64, f64)>,
    pub height: f64,
    u_range: (f64, f64),
    v_range: (f64, f64),
}

impl Footprint {
    pub fn new(ring: Vec<(f64, f64)>, height: f64) -> Footprint {
        let mut u_range = (f64::INFINITY, f64::NEG_INFINITY);
        let mut v_range = (f64::INFINITY, f64::NEG_INFINITY);
        for &(u, v) in &ring {
            u_range = (u_range.0.min(u), u_range.1.max(u));
            v_range = (v_range.0.min(v), v_range.1.max(v));
        }
        Footprint {
            ring,
            height,
            u_range,
            v_range,
        }
    }
}

/// Indices into `footprints`, furthest first. `squash` and `rise` come from the projection.
pub fn back_to_front(footprints: &[Footprint], squash: f64, rise: f64) -> Vec<usize> {
    let mut behind_count = vec![0usize; footprints.len()];
    let mut in_front: Vec<Vec<usize>> = vec![Vec::new(); footprints.len()];
    for (back, front) in occluding_pairs(footprints, squash, rise) {
        behind_count[front] += 1;
        in_front[back].push(front);
    }

    let key = |index: usize| (footprints[index].v_range.1 * KEY_PER_M) as i64;
    let mut ready: BinaryHeap<(i64, Reverse<usize>)> = (0..footprints.len())
        .filter(|&index| behind_count[index] == 0)
        .map(|index| (key(index), Reverse(index)))
        .collect();
    let mut is_done = vec![false; footprints.len()];
    let mut order = Vec::with_capacity(footprints.len());
    while order.len() < footprints.len() {
        let next = match ready.pop() {
            Some((_, Reverse(index))) => index,
            // A cycle, which overlapping concave shapes can make: draw the furthest one left.
            None => (0..footprints.len())
                .filter(|&index| !is_done[index])
                .max_by_key(|&index| key(index))
                .unwrap_or(0),
        };
        if is_done[next] {
            continue;
        }
        is_done[next] = true;
        order.push(next);
        for &front in &in_front[next] {
            behind_count[front] = behind_count[front].saturating_sub(1);
            if behind_count[front] == 0 && !is_done[front] {
                ready.push((key(front), Reverse(front)));
            }
        }
    }

    order
}

/// (back, front) for every pair where the front one may cover the back one.
fn occluding_pairs(footprints: &[Footprint], squash: f64, rise: f64) -> Vec<(usize, usize)> {
    let mut strips: HashMap<i64, Vec<usize>> = HashMap::new();
    for (index, footprint) in footprints.iter().enumerate() {
        for strip in strip_of(footprint.u_range.0)..=strip_of(footprint.u_range.1) {
            strips.entry(strip).or_default().push(index);
        }
    }

    let mut pairs = Vec::new();
    for (&strip, members) in &strips {
        for (position, &first) in members.iter().enumerate() {
            for &second in &members[position + 1..] {
                let (a, b) = (&footprints[first], &footprints[second]);
                // Each pair is looked at once, in the strip where both first appear.
                if strip_of(a.u_range.0.max(b.u_range.0)) != strip {
                    continue;
                }
                if !overlaps_on_paper(a, b, squash, rise) {
                    continue;
                }
                match is_in_front(a, b) {
                    Some(true) => pairs.push((second, first)),
                    Some(false) => pairs.push((first, second)),
                    None => {}
                }
            }
        }
    }

    pairs
}

fn strip_of(u: f64) -> i64 {
    (u / STRIP_M).floor() as i64
}

/// The two drawings share some paper: across, and up once heights are counted.
fn overlaps_on_paper(a: &Footprint, b: &Footprint, squash: f64, rise: f64) -> bool {
    let shares_u = a.u_range.0 < b.u_range.1 - TOUCH_M && b.u_range.0 < a.u_range.1 - TOUCH_M;
    let up = |footprint: &Footprint| {
        (
            footprint.v_range.0 * squash,
            footprint.v_range.1 * squash + footprint.height * rise,
        )
    };
    let (a_low, a_high) = up(a);
    let (b_low, b_high) = up(b);

    shares_u && a_low < b_high && b_low < a_high
}

/// True when `a` is nearer the viewer than `b` along a shared line of sight.
fn is_in_front(a: &Footprint, b: &Footprint) -> Option<bool> {
    let low = a.u_range.0.max(b.u_range.0);
    let high = a.u_range.1.min(b.u_range.1);
    for share in CUT_SHARES {
        let u = low + (high - low) * share;
        if let (Some(a_cut), Some(b_cut)) = (cut_at(&a.ring, u), cut_at(&b.ring, u)) {
            return Some(a_cut.0 + a_cut.1 < b_cut.0 + b_cut.1);
        }
    }

    None
}

/// The nearest and furthest `v` where the line at `u` crosses the ring.
fn cut_at(ring: &[(f64, f64)], u: f64) -> Option<(f64, f64)> {
    let mut range: Option<(f64, f64)> = None;
    for pair in ring.windows(2) {
        let ((u1, v1), (u2, v2)) = (pair[0], pair[1]);
        let crosses = (u1 <= u && u < u2) || (u2 <= u && u < u1);
        if !crosses {
            continue;
        }
        let v = v1 + (v2 - v1) * (u - u1) / (u2 - u1);
        range = Some(match range {
            Some((near, far)) => (near.min(v), far.max(v)),
            None => (v, v),
        });
    }

    range
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(u: f64, v: f64, side: f64, height: f64) -> Footprint {
        Footprint::new(
            vec![
                (u, v),
                (u + side, v),
                (u + side, v + side),
                (u, v + side),
                (u, v),
            ],
            height,
        )
    }

    #[test]
    fn the_block_further_back_is_drawn_first() {
        let footprints = [square(0.0, 0.0, 10.0, 20.0), square(0.0, 30.0, 10.0, 20.0)];
        assert_eq!(back_to_front(&footprints, 0.5, 0.8), vec![1, 0]);
    }

    #[test]
    fn a_small_block_behind_the_near_end_of_a_long_diagonal_slab_is_drawn_first() {
        // A thin slab from (0, 0) to (100, 100), and a small block just behind its near end.
        let slab = Footprint::new(
            vec![
                (0.0, 0.0),
                (100.0, 100.0),
                (98.0, 102.0),
                (-2.0, 2.0),
                (0.0, 0.0),
            ],
            30.0,
        );
        let small = square(8.0, 40.0, 6.0, 6.0);
        // The slab reaches much further back overall, which fools a sort by
        // depth; cutting both along the same line of sight gets it right.
        let order = back_to_front(&[small, slab], 0.5, 0.8);
        assert_eq!(order, vec![0, 1]);
    }
}
