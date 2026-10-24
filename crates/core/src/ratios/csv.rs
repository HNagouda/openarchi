//! Ratios as CSV, one row per class, for a spreadsheet or a report table.

use std::fmt::Write;

use super::{Ratios, Unit};

// The source column repeats per row, so every row still says where it came from once filtered.
const HEADER: &str = "group,class,value,unit,share_percent,source";
const SITE_GROUP: &str = "Site";

/// Site totals first, then every group's rows. Areas are whole square metres
/// and shares are percentages with one decimal, so the file reads without formatting.
pub fn to_csv(ratios: &Ratios) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{HEADER}");
    let _ = writeln!(
        out,
        "{SITE_GROUP},Site area,{:.0},m2,,",
        ratios.site_area_m2
    );
    let _ = writeln!(
        out,
        "{SITE_GROUP},Buildings,{},buildings,,",
        ratios.building_count
    );
    let _ = writeln!(
        out,
        "{SITE_GROUP},Building coverage,{:.3},ratio,,",
        ratios.coverage
    );
    let _ = writeln!(
        out,
        "{SITE_GROUP},Gross floor area,{:.0},m2,,",
        ratios.gross_floor_area_m2
    );
    let _ = writeln!(
        out,
        "{SITE_GROUP},Floor area ratio,{:.2},ratio,,",
        ratios.floor_area_ratio
    );
    if let Some(ratio) = ratios.bag_floor_area_ratio {
        let _ = writeln!(
            out,
            "{SITE_GROUP},Floor area ratio (BAG),{ratio:.2},ratio,,"
        );
    }
    if let Some(dwellings) = ratios.dwellings {
        let _ = writeln!(out, "{SITE_GROUP},Dwellings,{dwellings},dwellings,,");
    }
    if let Some(residents) = ratios.residents {
        let _ = writeln!(out, "{SITE_GROUP},Residents,{residents:.0},residents,,");
    }

    for group in &ratios.groups {
        for row in &group.rows {
            let _ = writeln!(
                out,
                "{},{},{:.0},{},{:.1},{}",
                field(&group.title),
                field(&row.label),
                row.value,
                unit_name(group.unit),
                row.share * 100.0,
                field(&group.source)
            );
        }
    }

    out
}

fn unit_name(unit: Unit) -> &'static str {
    match unit {
        Unit::SquareMetres => "m2",
        Unit::Buildings => "buildings",
        Unit::Parcels => "parcels",
        Unit::Places => "places",
        Unit::Stops => "stops",
    }
}

/// Quoted when it holds a comma or a quote, as RFC 4180 asks.
fn field(text: &str) -> String {
    if text.contains([',', '"', '\n']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_field_with_a_comma_is_quoted() {
        assert_eq!(field("500–1,000 m²"), "\"500–1,000 m²\"");
        assert_eq!(field("Grass"), "Grass");
    }
}
