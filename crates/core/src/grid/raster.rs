//! A square grid over the ground, and which of its cells a shape covers.
//!
//! A cell is covered when its centre lies inside the shape, found row by row
//! where the shape's edges cross the row's centre line.

use geo::Rect;

/// The grid: its south-west corner, cell size and cell counts. Cells are
/// numbered row by row from the south-west.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Raster {
    pub min: (f64, f64),
    pub cell_m: f64,
    pub columns: usize,
    pub rows: usize,
}

impl Raster {
    /// A grid of `cell_m` cells from the south-west corner of `rect`, enough to cover it.
    pub fn over(rect: Rect<f64>, cell_m: f64) -> Raster {
        Raster {
            min: (rect.min().x, rect.min().y),
            cell_m,
            columns: (rect.width() / cell_m).ceil() as usize,
            rows: (rect.height() / cell_m).ceil() as usize,
        }
    }

    /// The same cells and `margin_m` more on every side, at least, in whole cells,
    /// so a cell of this grid lies exactly on a cell of the grown one.
    pub fn grown(&self, margin_m: f64) -> Raster {
        let margin = (margin_m / self.cell_m).ceil() as usize;
        Raster {
            min: (
                self.min.0 - margin as f64 * self.cell_m,
                self.min.1 - margin as f64 * self.cell_m,
            ),
            cell_m: self.cell_m,
            columns: self.columns + 2 * margin,
            rows: self.rows + 2 * margin,
        }
    }

    /// The cell holding a point, or `None` past the grid.
    pub fn index_at(&self, (x, y): (f64, f64)) -> Option<usize> {
        let column = ((x - self.min.0) / self.cell_m).floor();
        let row = ((y - self.min.1) / self.cell_m).floor();
        let is_inside =
            column >= 0.0 && row >= 0.0 && column < self.columns as f64 && row < self.rows as f64;

        is_inside.then(|| row as usize * self.columns + column as usize)
    }

    pub fn len(&self) -> usize {
        self.columns * self.rows
    }

    /// The centre of cell `index`, in ground metres.
    pub fn centre(&self, index: usize) -> (f64, f64) {
        let column = index % self.columns;
        let row = index / self.columns;
        (
            self.min.0 + (column as f64 + 0.5) * self.cell_m,
            self.min.1 + (row as f64 + 0.5) * self.cell_m,
        )
    }

    /// Calls `mark` with every cell whose centre is inside the rings, counted
    /// even-odd, so the holes of a polygon stay uncovered.
    pub fn fill_rings(&self, rings: &[Vec<(f64, f64)>], mut mark: impl FnMut(usize)) {
        let Some((low_y, high_y)) = y_extent(rings.iter().flatten()) else {
            return;
        };
        let mut crossings = Vec::new();
        for row in self.rows_between(low_y, high_y) {
            let y = self.row_centre(row);
            crossings.clear();
            for ring in rings {
                for pair in ring.windows(2) {
                    if let Some(x) = crossing(pair[0], pair[1], y) {
                        crossings.push(x);
                    }
                }
            }
            crossings.sort_by(f64::total_cmp);
            let (spans, _) = crossings.as_chunks::<2>();
            for [left, right] in spans {
                self.mark_span(row, *left, *right, &mut mark);
            }
        }
    }

    /// Calls `mark` with every cell whose centre is inside a convex polygon,
    /// given as its corners without repeating the first. Faster than
    /// `fill_rings`, because each row is one span from the leftmost to the
    /// rightmost crossing.
    pub fn fill_convex(&self, corners: &[(f64, f64)], mut mark: impl FnMut(usize)) {
        let Some((low_y, high_y)) = y_extent(corners.iter()) else {
            return;
        };
        for row in self.rows_between(low_y, high_y) {
            let y = self.row_centre(row);
            let mut left = f64::INFINITY;
            let mut right = f64::NEG_INFINITY;
            for index in 0..corners.len() {
                let next = corners[(index + 1) % corners.len()];
                if let Some(x) = crossing(corners[index], next, y) {
                    left = left.min(x);
                    right = right.max(x);
                }
            }
            if left <= right {
                self.mark_span(row, left, right, &mut mark);
            }
        }
    }

    fn row_centre(&self, row: usize) -> f64 {
        self.min.1 + (row as f64 + 0.5) * self.cell_m
    }

    /// The rows whose centre lines lie between `low_y` and `high_y`, cut to the grid.
    fn rows_between(&self, low_y: f64, high_y: f64) -> std::ops::Range<usize> {
        let first = ((low_y - self.min.1) / self.cell_m - 0.5).ceil().max(0.0);
        let last = ((high_y - self.min.1) / self.cell_m - 0.5).floor() + 1.0;
        let last = last.clamp(0.0, self.rows as f64);
        (first as usize).min(self.rows)..last as usize
    }

    /// Marks the cells of `row` whose centres lie between `left` and `right`.
    fn mark_span(&self, row: usize, left: f64, right: f64, mark: &mut impl FnMut(usize)) {
        let first = ((left - self.min.0) / self.cell_m - 0.5).ceil().max(0.0);
        let last = ((right - self.min.0) / self.cell_m - 0.5).floor() + 1.0;
        let last = last.clamp(0.0, self.columns as f64) as usize;
        let start = row * self.columns;
        for column in (first as usize)..last {
            mark(start + column);
        }
    }
}

/// Where the edge from `a` to `b` crosses the line at height `y`. The lower
/// end counts and the upper end does not, so a corner on the line counts once.
fn crossing(a: (f64, f64), b: (f64, f64), y: f64) -> Option<f64> {
    let is_crossing = (a.1 <= y && y < b.1) || (b.1 <= y && y < a.1);
    if !is_crossing {
        return None;
    }

    Some(a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0))
}

fn y_extent<'a>(points: impl Iterator<Item = &'a (f64, f64)>) -> Option<(f64, f64)> {
    points.fold(None, |extent, &(_, y)| match extent {
        None => Some((y, y)),
        Some((low, high)) => Some((f64::min(low, y), f64::max(high, y))),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10 by 10 grid of 1 m cells from the origin.
    fn grid() -> Raster {
        Raster {
            min: (0.0, 0.0),
            cell_m: 1.0,
            columns: 10,
            rows: 10,
        }
    }

    fn covered(fill: impl FnOnce(&Raster, &mut dyn FnMut(usize))) -> usize {
        let raster = grid();
        let mut count = 0;
        fill(&raster, &mut |_| count += 1);
        count
    }

    #[test]
    fn a_square_covers_the_cells_whose_centres_it_holds() {
        let square = [(2.0, 2.0), (6.0, 2.0), (6.0, 5.0), (2.0, 5.0)];
        assert_eq!(covered(|r, mark| r.fill_convex(&square, mark)), 12);
    }

    #[test]
    fn a_hole_stays_uncovered() {
        let outer = vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (0.0, 0.0),
        ];
        let hole = vec![(3.0, 3.0), (7.0, 3.0), (7.0, 7.0), (3.0, 7.0), (3.0, 3.0)];
        assert_eq!(covered(|r, mark| r.fill_rings(&[outer, hole], mark)), 84);
    }

    #[test]
    fn a_shape_past_the_grid_is_cut_to_it() {
        let wide = [(-5.0, -5.0), (20.0, -5.0), (20.0, 1.0), (-5.0, 1.0)];
        assert_eq!(covered(|r, mark| r.fill_convex(&wide, mark)), 10);
    }
}
