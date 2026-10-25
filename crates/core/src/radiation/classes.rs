//! The solar radiation's classes, set per site. The diagram's scheme holds
//! its breaks as shares of the yearly total that open ground receives on the
//! site, from the site's own typical year. A sunnier country then still
//! spreads over every band, where fixed kWh/m² breaks set for the Netherlands
//! would put a site in Spain in the top band throughout. `sited` turns the
//! shares into kWh/m² breaks and labels for one site before the sheet is drawn.

use crate::diagram::{Classify, Diagram, Scheme};
use crate::ratios::grouped;
use crate::scene::{Layer, Scene};

/// The kWh/m² breaks are rounded to this, so the legend reads 310, not 307.
const ROUND_KWH: f64 = 10.0;
const PERCENT: f64 = 100.0;

/// `diagram` with its solar radiation classes in this site's kWh/m², or
/// `None` when it does not classify the radiation or the scene has no typical year.
pub fn sited(diagram: &Diagram, scene: &Scene) -> Option<Diagram> {
    let classify = diagram.classify.as_ref()?;
    if classify.layer != Layer::SolarRadiation {
        return None;
    }
    let solar = scene.solar.as_ref()?;
    let Scheme::Sequential { breaks: shares, .. } = &classify.scheme else {
        return None;
    };

    let open_kwh = super::open_ground_kwh(scene, solar);
    let breaks: Vec<f64> = shares
        .iter()
        .map(|share| (share * open_kwh / ROUND_KWH).round() * ROUND_KWH)
        .collect();
    let labels = labels(&breaks, shares);
    let note = classify.note.as_ref().map(|note| {
        format!(
            "Shares of the {} kWh/m² open ground receives here. {note}",
            grouped(open_kwh)
        )
    });

    let mut sited = diagram.clone();
    sited.classify = Some(Classify {
        scheme: Scheme::Sequential { breaks, labels },
        note,
        ..classify.clone()
    });
    Some(sited)
}

/// "Under 310 (under 30 %)", "310–520 (30–50 %)", …, "980 or more (95 % or more)".
fn labels(breaks: &[f64], shares: &[f64]) -> Vec<String> {
    let kwh: Vec<String> = breaks.iter().map(|value| grouped(*value)).collect();
    let percent: Vec<String> = shares
        .iter()
        .map(|share| format!("{:.0}", share * PERCENT))
        .collect();
    let Some(last) = breaks.len().checked_sub(1) else {
        return Vec::new();
    };

    let mut labels = vec![format!("Under {} (under {} %)", kwh[0], percent[0])];
    for index in 0..last {
        labels.push(format!(
            "{}–{} ({}–{} %)",
            kwh[index],
            kwh[index + 1],
            percent[index],
            percent[index + 1]
        ));
    }
    labels.push(format!(
        "{} or more ({} % or more)",
        kwh[last], percent[last]
    ));

    labels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_give_the_kwh_range_and_its_share() {
        let labels = labels(&[310.0, 520.0, 1000.0], &[0.3, 0.5, 0.95]);

        assert_eq!(
            labels,
            [
                "Under 310 (under 30 %)",
                "310–520 (30–50 %)",
                "520–1,000 (50–95 %)",
                "1,000 or more (95 % or more)",
            ]
        );
    }
}
