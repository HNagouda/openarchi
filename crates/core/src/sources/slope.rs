//! Terrain slope from the AHN terrain model, in percent.
//!
//! Percent rather than degrees, because that is how ramps and paths are
//! specified: 5 % is the steepest walk that needs no landings, 8 % roughly the
//! 1 in 12 a wheelchair ramp may have. It is read from the terrain the way
//! the contours read it (`contours::prepared`: 5 m cells, holes under
//! buildings filled, smoothed), so a kerb or a quay wall does not read as a cliff.

use crate::sources::raster::Grid;

/// Slope in percent per cell of `ground`, the terrain as `contours::prepared`
/// leaves it, from the 3 x 3 cells around each one (Horn's method). Edge
/// cells reuse their nearest neighbours.
pub fn slope_percent(ground: &Grid) -> Grid {
    let at = |column: isize, row: isize| {
        let column = column.clamp(0, ground.width as isize - 1) as usize;
        let row = row.clamp(0, ground.height as isize - 1) as usize;
        f64::from(ground.value(column, row))
    };

    let mut values = Vec::with_capacity(ground.values.len());
    for row in 0..ground.height as isize {
        for column in 0..ground.width as isize {
            let east =
                at(column + 1, row - 1) + 2.0 * at(column + 1, row) + at(column + 1, row + 1);
            let west =
                at(column - 1, row - 1) + 2.0 * at(column - 1, row) + at(column - 1, row + 1);
            let south =
                at(column - 1, row + 1) + 2.0 * at(column, row + 1) + at(column + 1, row + 1);
            let north =
                at(column - 1, row - 1) + 2.0 * at(column, row - 1) + at(column + 1, row - 1);
            let rise_east = (east - west) / (8.0 * ground.cell_m);
            let rise_north = (north - south) / (8.0 * ground.cell_m);
            values.push((100.0 * rise_east.hypot(rise_north)) as f32);
        }
    }

    Grid {
        west: ground.west,
        north: ground.north,
        cell_m: ground.cell_m,
        width: ground.width,
        height: ground.height,
        values,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::contours::prepared;

    #[test]
    fn a_one_in_twenty_slope_reads_as_five_percent() {
        // 100 by 100 m at 0.5 m cells, rising 1 m per 20 m northwards.
        let (width, height) = (200, 200);
        let values = (0..width * height)
            .map(|index| ((height - index / width) as f32 * 0.5) / 20.0)
            .collect();
        let terrain = Grid {
            west: 0.0,
            north: 100.0,
            cell_m: 0.5,
            width,
            height,
            values,
        };
        let slope = slope_percent(&prepared(&terrain));
        let middle = slope.value(slope.width / 2, slope.height / 2);
        assert!((middle - 5.0).abs() < 0.1, "{middle}");
    }
}
