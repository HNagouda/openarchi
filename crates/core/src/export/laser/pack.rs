//! Laying pieces out on laser beds, by their bounding boxes.
//!
//! A guillotine packer: every bed starts as one free rectangle, and each piece
//! takes the top-left corner of the first free rectangle it fits, splitting the
//! rest into a strip to its right (as tall as the piece) and the full width
//! below. Pieces go tallest first, so the strips fill in rows. It wastes some
//! board next to irregular footprints, but it is short, it never overlaps, and
//! the same site always gives the same sheets.

use crate::{Error, Result};

/// Where one piece lands: which bed, and its top-left corner inside the bed's
/// usable area, in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub sheet: usize,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy)]
struct Free {
    sheet: usize,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

/// A placement for every piece of `sizes` (width, height), in the same order,
/// on beds whose usable area is `usable` (width, height), `gap` apart.
pub fn pack(sizes: &[(f64, f64)], usable: (f64, f64), gap: f64) -> Result<Vec<Placement>> {
    for &(width, height) in sizes {
        if width > usable.0 || height > usable.1 {
            return Err(Error::Invalid(format!(
                "A piece of {width:.0} × {height:.0} mm does not fit on the bed's usable {:.0} × {:.0} mm.",
                usable.0, usable.1
            )));
        }
    }

    // Tallest first, then widest, then in the order given, so the result never depends on sort stability.
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| {
        sizes[b]
            .1
            .total_cmp(&sizes[a].1)
            .then(sizes[b].0.total_cmp(&sizes[a].0))
            .then(a.cmp(&b))
    });
    // Every piece carries its gap on its right and bottom, so the bed gets one gap more to hold the last one.
    let bed = (usable.0 + gap, usable.1 + gap);
    let smallest = sizes.iter().fold((f64::MAX, f64::MAX), |least, &(w, h)| {
        (least.0.min(w + gap), least.1.min(h + gap))
    });

    let mut free: Vec<Free> = Vec::new();
    let mut sheet_count = 0;
    let mut placements = vec![
        Placement {
            sheet: 0,
            x: 0.0,
            y: 0.0
        };
        sizes.len()
    ];
    for index in order {
        let (width, height) = (sizes[index].0 + gap, sizes[index].1 + gap);
        let chosen = match first_fit(&free, width, height) {
            Some(chosen) => chosen,
            None => {
                free.push(Free {
                    sheet: sheet_count,
                    x: 0.0,
                    y: 0.0,
                    width: bed.0,
                    height: bed.1,
                });
                sheet_count += 1;
                free.len() - 1
            }
        };

        let space = free.swap_remove(chosen);
        placements[index] = Placement {
            sheet: space.sheet,
            x: space.x,
            y: space.y,
        };
        let right = Free {
            x: space.x + width,
            width: space.width - width,
            height,
            ..space
        };
        let below = Free {
            y: space.y + height,
            height: space.height - height,
            ..space
        };
        // A scrap no piece could ever fill is forgotten, which keeps the search short.
        for rest in [right, below] {
            if rest.width >= smallest.0 && rest.height >= smallest.1 {
                free.push(rest);
            }
        }
    }

    Ok(placements)
}

/// The free rectangle a piece of this size goes in: the first bed, then the
/// topmost, then the leftmost that holds it.
fn first_fit(free: &[Free], width: f64, height: f64) -> Option<usize> {
    free.iter()
        .enumerate()
        .filter(|(_, space)| space.width >= width && space.height >= height)
        .min_by(|(_, a), (_, b)| {
            a.sheet
                .cmp(&b.sheet)
                .then(a.y.total_cmp(&b.y))
                .then(a.x.total_cmp(&b.x))
        })
        .map(|(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    const USABLE: (f64, f64) = (590.0, 390.0);
    const GAP: f64 = 2.0;

    /// A mix like a real site: one big base plate and many small pieces of many sizes.
    fn sizes() -> Vec<(f64, f64)> {
        let mut sizes = vec![(300.0, 280.0)];
        for index in 0..600 {
            let width = 3.0 + (index * 37 % 41) as f64;
            let height = 2.0 + (index * 53 % 29) as f64;
            sizes.push((width, height));
        }

        sizes
    }

    #[test]
    fn packed_pieces_stay_on_the_bed_and_never_overlap() {
        let sizes = sizes();
        let placements = pack(&sizes, USABLE, GAP).unwrap();
        for (index, (place, size)) in placements.iter().zip(&sizes).enumerate() {
            assert!(place.x >= 0.0 && place.y >= 0.0, "{index}");
            assert!(place.x + size.0 <= USABLE.0 + 1e-9, "{index} {place:?}");
            assert!(place.y + size.1 <= USABLE.1 + 1e-9, "{index} {place:?}");
            for other in index + 1..sizes.len() {
                let (theirs, their_size) = (placements[other], sizes[other]);
                if theirs.sheet != place.sheet {
                    continue;
                }
                let apart = place.x + size.0 + GAP <= theirs.x + 1e-9
                    || theirs.x + their_size.0 + GAP <= place.x + 1e-9
                    || place.y + size.1 + GAP <= theirs.y + 1e-9
                    || theirs.y + their_size.1 + GAP <= place.y + 1e-9;
                assert!(apart, "{index} and {other} overlap");
            }
        }
    }

    #[test]
    fn small_pieces_fill_the_room_beside_the_base_plate() {
        let placements = pack(&sizes(), USABLE, GAP).unwrap();
        assert_eq!(
            placements[0],
            Placement {
                sheet: 0,
                x: 0.0,
                y: 0.0
            }
        );
        let beside = placements
            .iter()
            .filter(|place| place.sheet == 0 && place.x >= 300.0 && place.y < 280.0)
            .count();
        assert!(beside > 50, "{beside}");
    }

    #[test]
    fn the_same_pieces_always_give_the_same_layout() {
        assert_eq!(
            pack(&sizes(), USABLE, GAP).unwrap(),
            pack(&sizes(), USABLE, GAP).unwrap()
        );
    }

    #[test]
    fn a_piece_larger_than_the_bed_is_a_readable_error() {
        let error = pack(&[(700.0, 10.0)], USABLE, GAP).unwrap_err();
        assert!(error.to_string().contains("does not fit"), "{error}");
    }

    #[test]
    fn a_full_bed_opens_the_next_one() {
        let placements = pack(&[(300.0, 390.0); 3], USABLE, GAP).unwrap();
        let sheets: Vec<usize> = placements.iter().map(|place| place.sheet).collect();
        assert_eq!(sheets, vec![0, 1, 2]);
    }
}
