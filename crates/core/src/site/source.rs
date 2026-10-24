use crate::scene::Credit;
use crate::sources::{ahn, bag, bgt, brk, cbs, liwo, osm, rivm};

/// Every data source of a scene, in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Source {
    Bgt,
    Bag,
    Brk,
    Ahn,
    Rivm,
    Liwo,
    Cbs,
    BagUnits,
    Osm,
    /// Buildings, roads, water and green around the site, see `site/context.rs`.
    Context,
}

pub(super) const SOURCES: [Source; 10] = [
    Source::Bgt,
    Source::Bag,
    Source::Brk,
    Source::Ahn,
    Source::Osm,
    Source::Rivm,
    Source::Liwo,
    Source::Cbs,
    Source::BagUnits,
    Source::Context,
];

/// The sources a site cannot do without. The others add layers and figures,
/// so when only they arrive the fetch still failed.
pub(super) const BASE_SOURCES: [Source; 4] = [Source::Bgt, Source::Bag, Source::Brk, Source::Ahn];

impl Source {
    pub(super) fn step(self) -> &'static str {
        match self {
            Source::Bgt => "Roads, water and green",
            Source::Bag => "Buildings",
            Source::Brk => "Parcels",
            Source::Ahn => "Elevation",
            Source::Rivm => "Noise and air quality",
            Source::Liwo => "Flood depth",
            Source::Cbs => "Population",
            Source::BagUnits => "Dwellings",
            Source::Osm => "Places, transit and walking routes",
            Source::Context => "Neighbourhood",
        }
    }

    /// `None` for a source already credited under another name.
    pub(super) fn credit(self) -> Option<Credit> {
        match self {
            Source::Bgt => Some(bgt::credit()),
            Source::Bag => Some(bag::credit()),
            Source::Brk => Some(brk::credit()),
            Source::Ahn => Some(ahn::credit()),
            Source::Rivm => Some(rivm::credit()),
            Source::Liwo => Some(liwo::credit()),
            Source::Cbs => Some(cbs::credit()),
            // The units are BAG, which the buildings already credit.
            Source::BagUnits => None,
            Source::Osm => Some(osm::credit()),
            // BGT, BAG and AHN again, which the site already credits.
            Source::Context => None,
        }
    }
}
