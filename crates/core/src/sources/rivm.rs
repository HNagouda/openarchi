//! RIVM's national maps of noise, air quality, tree height, green classes and
//! the urban heat island, from the Atlas Leefomgeving WCS, with their credits.
//!
//! RIVM publishes each map again under a new dated id when a new year is
//! modelled: alo__rivm_20250101_Geluid_lden_wegverkeer_2022 was published on
//! 2025-01-01 and describes 2022. The service also offers "actueel" aliases,
//! but the legend has to say which year it shows, so openarchi reads the list
//! of coverages when a site is fetched and takes the newest dated id of each
//! map: the latest year, and of that year the latest publication.

use crate::scene::{Credit, Layer};
use crate::sources::edition::{Edition, ASK_TIMEOUT};
use crate::sources::wcs::{self, Coverage};
use crate::Result;

const SERVICE: &str = "RIVM";
const WCS_URL: &str = "https://data.rivm.nl/geo/wcs";
const AXES: (&str, &str) = ("X", "Y");
/// The service holds about 15,000 coverages of many atlases; only these are RIVM's own maps.
const ID_PREFIX: &str = "alo__rivm_";
/// The publication date in an id, "20250101".
const DATE_DIGITS: usize = 8;
const YEAR_DIGITS: usize = 4;

/// One RIVM map, by the shape of its dated ids: `before_date`, the
/// publication date, `before_year`, the year it describes, and nothing after.
/// That last rule keeps out variants such as "..._gm_NO22019_Int_v2".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Map {
    before_date: &'static str,
    before_year: &'static str,
    /// The newest edition on 2026-10-24, used when the service cannot be asked.
    pub known: &'static str,
}

/// Road traffic noise, Lden in dB, 10 m cells, stored as whole decibels.
pub const ROAD_NOISE: Map = Map {
    before_date: "alo__rivm_",
    before_year: "_Geluid_lden_wegverkeer_",
    known: "alo__rivm_20250101_Geluid_lden_wegverkeer_2022",
};
/// Every source together (road, rail, air, industry, wind turbines), Lden in dB.
pub const ALL_NOISE: Map = Map {
    before_date: "alo__rivm_",
    before_year: "_Geluid_lden_allebronnen_",
    known: "alo__rivm_20250801_Geluid_lden_allebronnen_2022",
};
/// Annual means in µg/m³, 25 m cells on a grid offset from round numbers.
pub const NO2: Map = Map {
    before_date: "alo__rivm_nsl_",
    before_year: "_gm_NO2",
    known: "alo__rivm_nsl_20260401_gm_NO22024",
};
pub const PM10: Map = Map {
    before_date: "alo__rivm_nsl_",
    before_year: "_gm_PM10",
    known: "alo__rivm_nsl_20260401_gm_PM102024",
};
pub const PM25: Map = Map {
    before_date: "alo__rivm_nsl_",
    before_year: "_gm_PM25",
    known: "alo__rivm_nsl_20260401_gm_PM252024",
};

/// Every map openarchi fetches, in one place.
pub const MAPS: [Map; 5] = [ROAD_NOISE, ALL_NOISE, NO2, PM10, PM25];

/// Tree height in whole metres above the ground, 5 m cells, for every tree
/// of 2.5 m or more; cells without a tree are nodata.
pub const TREE_HEIGHT: Map = Map {
    before_date: "alo__rivm_",
    before_year: "_boomhoogte_",
    known: "alo__rivm_20240101_boomhoogte_2022",
};
/// Green land in 5 m cells, by what protects it: 1 farmland, 2 Natura 2000,
/// 3 the Nature Network (Natuurnetwerk Nederland), 4 anywhere else. Cells
/// without green are nodata. The codes are in the WMS legend of the layer.
pub const GREEN_CLASSES: Map = Map {
    before_date: "alo__rivm_",
    before_year: "_groenkaart_",
    known: "alo__rivm_20240101_groenkaart_2022",
};
/// The green maps, in one place, kept apart from `MAPS` because they come
/// from another atlas page and fail on their own.
pub const GREEN_MAPS: [Map; 2] = [TREE_HEIGHT, GREEN_CLASSES];

/// The urban heat island effect in °C, 10 m cells: how much warmer the air is
/// over a summer (June to August) than in the countryside around, from RIVM's
/// model. RIVM's own WCS has it under an undated id outside the "alo__rivm_"
/// maps, so the edition is pinned, like LIWO's.
// #! NOTE[heat, source] : RIVM's own coverage rather than the Klimaateffectatlas copy, see docs/data-sources.md; switch if the atlas version is the one to quote?
pub const HEAT_ISLAND_ID: &str = "ank__Stedelijk_hitte_eiland_effect_01062022_v2";
/// The year the heat island map was published, from its id. It describes no
/// single year: the model runs on land use and green maps of around 2020.
pub const HEAT_ISLAND_YEAR: i32 = 2022;

/// The coverage of one edition of a map.
pub fn coverage(id: &str) -> Coverage<'_> {
    Coverage {
        service: SERVICE,
        url: WCS_URL,
        id,
        axes: AXES,
        scale_factor: None,
    }
}

/// Every RIVM map id the service lists now, dated or not.
pub fn list_ids() -> Result<Vec<String>> {
    let ids = wcs::list_coverage_ids(SERVICE, WCS_URL, ASK_TIMEOUT)?;
    Ok(ids
        .into_iter()
        .filter(|id| id.starts_with(ID_PREFIX))
        .collect())
}

/// The newest edition of `map` among `ids`, or `None` when none has its shape.
pub fn newest(map: &Map, ids: &[String]) -> Option<Edition> {
    let ((_, year), id) = ids
        .iter()
        .filter_map(|id| Some((parse(map, id)?, id)))
        // The latest year, and of that year the latest publication.
        .max_by_key(|((published, year), _)| (*year, *published))?;

    Some(Edition {
        year,
        id: id.clone(),
    })
}

/// The edition named in code, for when the service cannot be asked.
pub fn known(map: &Map) -> Edition {
    // A broken constant, caught by `every_known_edition_has_the_shape_of_its_map`.
    let (_, year) = parse(map, map.known).expect("a known RIVM id has the shape of its map");
    Edition {
        year,
        id: map.known.to_string(),
    }
}

/// The publication date and the year of a dated id of `map`:
/// "alo__rivm_nsl_20260401_gm_NO22024" -> (20260401, 2024).
fn parse(map: &Map, id: &str) -> Option<(u32, i32)> {
    let rest = id.strip_prefix(map.before_date)?;
    let date = rest.get(..DATE_DIGITS)?;
    let year = rest.get(DATE_DIGITS..)?.strip_prefix(map.before_year)?;
    let is_digits = |text: &str| text.bytes().all(|byte| byte.is_ascii_digit());
    if year.len() != YEAR_DIGITS || !is_digits(date) || !is_digits(year) {
        return None;
    }

    Some((date.parse().ok()?, year.parse().ok()?))
}

// data.overheid.nl lists both the noise and the air quality maps as public
// domain, with RIVM as the owner. The WCS capabilities say nothing about it.
pub fn credit() -> Credit {
    Credit {
        name: "RIVM Atlas Leefomgeving, noise and air quality maps".to_string(),
        license: "Public Domain Mark 1.0".to_string(),
        url: "https://www.atlasleefomgeving.nl".to_string(),
        layers: vec![
            Layer::RoadNoise,
            Layer::AllNoise,
            Layer::No2,
            Layer::Pm10,
            Layer::Pm25,
        ],
    }
}

// data.overheid.nl lists the tree height and green maps as public domain,
// with RIVM as the owner, like the noise maps.
pub fn green_credit() -> Credit {
    Credit {
        name: "RIVM Atlas Leefomgeving, tree height and green maps".to_string(),
        license: "Public Domain Mark 1.0".to_string(),
        url: "https://www.atlasleefomgeving.nl".to_string(),
        layers: vec![Layer::TreeCanopy, Layer::GreenCover],
    }
}

// data.overheid.nl lists "Stedelijk hitte-eiland effect (UHI) in Nederland" as
// public domain, made by RIVM for the Atlas Natuurlijk Kapitaal.
pub fn heat_credit() -> Credit {
    Credit {
        name: "RIVM, urban heat island effect (UHI)".to_string(),
        license: "Public Domain Mark 1.0".to_string(),
        url: "https://www.atlasleefomgeving.nl/stedelijk-hitte-eiland-effect-uhi".to_string(),
        layers: vec![Layer::HeatIsland],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The road noise, all sources and air quality entries of the capabilities
    /// of 2026-10-24, with "actueel" aliases, interpolated variants and one
    /// coverage of another atlas left in.
    const CAPABILITIES: &str = include_str!("../../tests/fixtures/rivm-capabilities.xml");

    fn listed() -> Vec<String> {
        wcs::coverage_ids(CAPABILITIES)
            .into_iter()
            .filter(|id| id.starts_with(ID_PREFIX))
            .collect()
    }

    #[test]
    fn the_newest_year_of_each_map_is_picked_from_the_capabilities() {
        let ids = listed();
        let years: Vec<(i32, String)> = MAPS
            .iter()
            .map(|map| newest(map, &ids).unwrap())
            .map(|edition| (edition.year, edition.id))
            .collect();

        assert_eq!(
            years,
            vec![
                (2022, ROAD_NOISE.known.to_string()),
                (2022, ALL_NOISE.known.to_string()),
                (2024, NO2.known.to_string()),
                (2024, PM10.known.to_string()),
                (2024, PM25.known.to_string()),
            ]
        );
    }

    #[test]
    fn a_later_publication_of_the_same_year_wins() {
        let ids = vec![
            "alo__rivm_20250101_Geluid_lden_wegverkeer_2022".to_string(),
            "alo__rivm_20260301_Geluid_lden_wegverkeer_2022".to_string(),
            "alo__rivm_20260101_Geluid_lden_wegverkeer_2021".to_string(),
        ];
        assert_eq!(newest(&ROAD_NOISE, &ids).unwrap().id, ids[1]);
    }

    #[test]
    fn aliases_and_variants_are_not_editions() {
        let ids = vec![
            "alo__rivm_Geluid_lden_wegverkeer_actueel".to_string(),
            "alo__rivm_nsl_20210101_gm_NO22019_Int_v2".to_string(),
        ];
        assert_eq!(newest(&ROAD_NOISE, &ids), None);
        assert_eq!(newest(&NO2, &ids), None);
    }

    #[test]
    fn the_green_maps_find_their_newest_year_and_skip_the_older_atlas() {
        let ids = vec![
            "alo__rivm_20240101_boomhoogte_2022".to_string(),
            "alo__rivm_20240101_groenkaart_2022".to_string(),
            // The 10 m share of green, in the older natural capital atlas.
            "ank__rivm_20231221_groenkaart_10m_2022".to_string(),
            "ank__rivm_087_20170415_gm_boomhoogte".to_string(),
        ];
        assert_eq!(newest(&TREE_HEIGHT, &ids).unwrap().id, ids[0]);
        assert_eq!(newest(&GREEN_CLASSES, &ids).unwrap().id, ids[1]);
    }

    #[test]
    fn every_known_edition_has_the_shape_of_its_map() {
        for map in MAPS.iter().chain(&GREEN_MAPS) {
            assert!(parse(map, map.known).is_some(), "{}", map.known);
        }
    }
}
