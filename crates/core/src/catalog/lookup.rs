//! Finding a built-in diagram by the id a look or an explode plate stores.
//!
//! Some diagrams were folded into another one as variants. Their old ids keep
//! working, so a project saved before the fold opens on the same drawing.

use crate::diagram::Diagram;
use crate::project::Look;
use crate::view::View;

/// Diagrams that became a variant of another, as (old id, diagram id, variant
/// id), in one place.
const FOLDED_DIAGRAMS: [(&str, &str, &str); 4] = [
    ("air-quality-pm10", "air-quality", "pm10"),
    ("air-quality-pm25", "air-quality", "pm25"),
    ("traffic-noise", "noise", "road"),
    ("noise-all-sources", "noise", "all"),
];

/// Splits a plate id that names one variant: "air-quality:pm10".
const VARIANT_SEPARATOR: char = ':';

/// "air-quality-pm10" -> ("air-quality", Some("pm10")). An id that was never
/// folded keeps the variant it came with.
pub fn current_id(id: &str, variant: Option<&str>) -> (String, Option<String>) {
    match FOLDED_DIAGRAMS.iter().find(|(old, _, _)| *old == id) {
        Some((_, diagram, folded)) => (diagram.to_string(), Some(folded.to_string())),
        None => (id.to_string(), variant.map(str::to_string)),
    }
}

/// The diagram `id` names, drawn for `variant`, following folded ids.
pub fn find_drawing(diagrams: &[Diagram], id: &str, variant: Option<&str>) -> Option<Diagram> {
    let (id, variant) = current_id(id, variant);
    diagrams
        .iter()
        .find(|diagram| diagram.id == id)
        .map(|diagram| diagram.drawn_as(variant.as_deref()))
}

/// A plate id is a diagram id, drawn as its first variant, or
/// "diagram:variant" for one variant. Anything else that names one drawing,
/// like the ratios reusing a legend's classes, uses the same form. A variant
/// the diagram does not have finds nothing, because the plate asked for it by name.
pub fn find_plate(diagrams: &[Diagram], plate: &str) -> Option<Diagram> {
    let Some((id, variant)) = plate.split_once(VARIANT_SEPARATOR) else {
        return find_drawing(diagrams, plate, None);
    };

    let diagram = diagrams.iter().find(|diagram| diagram.id == id)?;
    let has_variant = diagram.variants.iter().any(|known| known.id == variant);
    has_variant.then(|| diagram.drawn_as(Some(variant)))
}

/// "air-quality" and "pm10" -> "air-quality:pm10"; no variant -> the diagram id.
pub fn plate_id(diagram_id: &str, variant: Option<&str>) -> String {
    match variant {
        Some(variant) => format!("{diagram_id}{VARIANT_SEPARATOR}{variant}"),
        None => diagram_id.to_string(),
    }
}

/// The look with every folded diagram id, its own and its plates', replaced
/// by the diagram and variant it became.
pub fn current_look(look: Look) -> Look {
    let (diagram_id, variant) = current_id(&look.diagram_id, look.variant.as_deref());
    let plates = look
        .view
        .plates
        .iter()
        .map(|plate| {
            let (id, variant) = current_id(plate, None);
            plate_id(&id, variant.as_deref())
        })
        .collect();

    Look {
        diagram_id,
        variant,
        view: View {
            plates,
            ..look.view
        },
        ..look
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::builtin_diagrams;

    #[test]
    fn every_folded_id_lands_on_a_real_variant() {
        let diagrams = builtin_diagrams();
        for (old, diagram_id, variant) in FOLDED_DIAGRAMS {
            let diagram = diagrams.iter().find(|d| d.id == diagram_id).unwrap();
            assert!(
                diagram.variants.iter().any(|v| v.id == variant),
                "{old} -> {diagram_id}:{variant}"
            );
        }
    }

    #[test]
    fn an_old_pm10_project_opens_on_air_quality_pm10() {
        let look = Look {
            diagram_id: "air-quality-pm10".to_string(),
            view: View {
                plates: vec!["context".to_string(), "noise-all-sources".to_string()],
                ..View::default()
            },
            ..Look::default()
        };

        let current = current_look(look);

        assert_eq!(current.diagram_id, "air-quality");
        assert_eq!(current.variant.as_deref(), Some("pm10"));
        assert_eq!(current.view.plates, ["context", "noise:all"]);
    }

    #[test]
    fn a_plate_can_name_one_variant() {
        let diagrams = builtin_diagrams();

        let pm25 = find_plate(&diagrams, "air-quality:pm25").unwrap();
        let first = find_plate(&diagrams, "air-quality").unwrap();

        assert_eq!(pm25.name, "Air quality, PM2.5");
        assert_eq!(first.name, "Air quality, NO₂");
        assert!(find_plate(&diagrams, "no-such-diagram").is_none());
        assert!(find_plate(&diagrams, "air-quality:ozone").is_none());
    }
}
