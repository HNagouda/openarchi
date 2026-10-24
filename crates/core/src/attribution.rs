//! Short source names for the one line under each ratio group and analysis
//! section: "Buildings from BAG, the ground from BGT."
//!
//! The names come from each source's own `Credit`, so a source is named once,
//! in its module, and the line under a chart always matches the sheet credits.

use crate::scene::Credit;

/// The shortest name a reader knows a source by. That is the first
/// all-capital word before the first comma, which is how registers are known
/// ("BAG", "RIVM", "KNMI"), or else the whole name up to that comma, without
/// a copyright sign. Words after the comma are the publisher or the portal
/// ("Kadaster, via PDOK"), not the source.
///
/// "Basisregistratie Adressen en Gebouwen (BAG), via PDOK" -> "BAG";
/// "© OpenStreetMap contributors" -> "OpenStreetMap contributors".
pub fn short_name(credit: &Credit) -> String {
    let title = credit.name.split(',').next().unwrap_or_default();
    let acronym = title.split_whitespace().find_map(|word| {
        let letters = word.trim_matches(|c: char| !c.is_alphanumeric());
        let is_acronym = letters.chars().count() >= 2
            && letters
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
            && letters.chars().any(|c| c.is_ascii_uppercase());
        is_acronym.then(|| letters.to_string())
    });
    match acronym {
        Some(acronym) => acronym,
        None => title.trim_start_matches('©').trim().to_string(),
    }
}

/// "RIVM (2022)", or just "RIVM" when the year is not known.
pub fn dated(credit: &Credit, year: Option<i32>) -> String {
    match year {
        Some(year) => format!("{} ({year})", short_name(credit)),
        None => short_name(credit),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sources::{ahn, bag, bgt, boundaries, brk, cbs, knmi, liwo, osm, pvgis, rivm};

    #[test]
    fn every_source_has_a_short_name_a_reader_knows() {
        let names: Vec<String> = [
            bag::credit(),
            bgt::credit(),
            brk::credit(),
            ahn::credit(),
            rivm::credit(),
            liwo::credit(),
            cbs::credit(),
            osm::credit(),
            pvgis::credit(),
            knmi::credit(),
            boundaries::credit(),
            boundaries::country_credit(),
        ]
        .iter()
        .map(short_name)
        .collect();
        assert_eq!(
            names,
            [
                "BAG",
                "BGT",
                "BRK",
                "AHN",
                "RIVM",
                "LIWO",
                "CBS",
                "OpenStreetMap contributors",
                "PVGIS",
                "KNMI",
                "Bestuurlijke gebieden",
                "CBS"
            ]
        );
    }

    #[test]
    fn a_known_year_follows_the_name() {
        assert_eq!(dated(&rivm::credit(), Some(2022)), "RIVM (2022)");
        assert_eq!(dated(&rivm::credit(), None), "RIVM");
    }
}
