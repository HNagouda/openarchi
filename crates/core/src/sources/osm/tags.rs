//! What an OpenStreetMap tag means for openarchi: which points are businesses
//! or facilities and in which group, which stops serve which mode, and which
//! ways can be walked.
//!
//! The tables below are the whole list. The Overpass query asks for exactly
//! these values, so a value missing here is never downloaded, and street
//! furniture (benches, waste baskets, vending machines, bicycle racks) never
//! arrives at all.

use std::collections::BTreeMap;

use crate::scene::Layer;

/// Business groups in legend order: (group, label).
pub const BUSINESS_GROUPS: [(&str, &str); 5] = [
    ("food_drink", "Food and drink"),
    ("shop", "Shops"),
    ("office", "Offices"),
    ("hotel", "Hotels"),
    ("service", "Services and crafts"),
];

/// Facility groups in legend order: (group, label).
pub const FACILITY_GROUPS: [(&str, &str); 7] = [
    ("education", "Education"),
    ("culture", "Culture"),
    ("civic", "Civic and worship"),
    ("health", "Health"),
    ("leisure", "Sport and play"),
    ("parking", "Car parking"),
    ("bicycle_parking", "Bicycle parking"),
];

/// The facility groups that are somewhere to leave a car or a bike rather
/// than somewhere to go. Walk counts and ratios keep them apart, so 400
/// bicycle racks do not drown out the schools, and a site without a car park
/// is not told it lacks a facility.
pub const PARKING_GROUPS: [&str; 2] = ["parking", "bicycle_parking"];

/// Transit modes, most important first: (group, label). A stop served by
/// several modes takes the first.
pub const TRANSIT_GROUPS: [(&str, &str); 5] = [
    ("train", "Train"),
    ("metro", "Metro"),
    ("tram", "Tram"),
    ("bus", "Bus"),
    ("ferry", "Ferry"),
];

/// Every `amenity` value openarchi keeps, with its layer and group.
pub const AMENITIES: [(&str, Layer, &str); 42] = [
    ("restaurant", Layer::Business, "food_drink"),
    ("cafe", Layer::Business, "food_drink"),
    ("bar", Layer::Business, "food_drink"),
    ("pub", Layer::Business, "food_drink"),
    ("fast_food", Layer::Business, "food_drink"),
    ("ice_cream", Layer::Business, "food_drink"),
    ("biergarten", Layer::Business, "food_drink"),
    ("food_court", Layer::Business, "food_drink"),
    ("nightclub", Layer::Business, "food_drink"),
    // A Dutch coffeeshop sells cannabis, so it counts as a shop.
    ("cannabis", Layer::Business, "shop"),
    ("marketplace", Layer::Business, "shop"),
    ("bank", Layer::Business, "service"),
    ("bureau_de_change", Layer::Business, "service"),
    ("car_rental", Layer::Business, "service"),
    ("veterinary", Layer::Business, "service"),
    ("brothel", Layer::Business, "service"),
    ("school", Layer::Facility, "education"),
    ("university", Layer::Facility, "education"),
    ("college", Layer::Facility, "education"),
    ("kindergarten", Layer::Facility, "education"),
    ("childcare", Layer::Facility, "education"),
    ("library", Layer::Facility, "education"),
    ("music_school", Layer::Facility, "education"),
    ("language_school", Layer::Facility, "education"),
    ("theatre", Layer::Facility, "culture"),
    ("cinema", Layer::Facility, "culture"),
    ("arts_centre", Layer::Facility, "culture"),
    ("concert_hall", Layer::Facility, "culture"),
    ("townhall", Layer::Facility, "civic"),
    ("police", Layer::Facility, "civic"),
    ("post_office", Layer::Facility, "civic"),
    ("place_of_worship", Layer::Facility, "civic"),
    ("community_centre", Layer::Facility, "civic"),
    ("courthouse", Layer::Facility, "civic"),
    ("fire_station", Layer::Facility, "civic"),
    ("social_facility", Layer::Facility, "civic"),
    ("doctors", Layer::Facility, "health"),
    ("pharmacy", Layer::Facility, "health"),
    ("hospital", Layer::Facility, "health"),
    ("clinic", Layer::Facility, "health"),
    ("dentist", Layer::Facility, "health"),
    // Racks, stands and the big station bicycle parks; `capacity` tells them apart.
    ("bicycle_parking", Layer::Facility, "bicycle_parking"),
];

/// `amenity=parking` on its own, because car parks are areas that the query
/// returns as their centre, and parking spaces and entrances are left out.
pub const PARKING_AMENITY: &str = "parking";

/// Every `tourism` value openarchi keeps.
pub const TOURISM: [(&str, Layer, &str); 7] = [
    ("hotel", Layer::Business, "hotel"),
    ("hostel", Layer::Business, "hotel"),
    ("guest_house", Layer::Business, "hotel"),
    ("motel", Layer::Business, "hotel"),
    ("apartment", Layer::Business, "hotel"),
    ("museum", Layer::Facility, "culture"),
    ("gallery", Layer::Facility, "culture"),
];

/// Every `leisure` value openarchi keeps. Gardens are left out: in a Dutch
/// city most mapped gardens are private back gardens.
pub const LEISURE: [&str; 11] = [
    "park",
    "playground",
    "sports_centre",
    "fitness_centre",
    "pitch",
    "swimming_pool",
    "stadium",
    "sports_hall",
    "dog_park",
    "ice_rink",
    "water_park",
];

/// `highway` values that cannot be walked, or are not built yet.
const NOT_WALKABLE: [&str; 10] = [
    "motorway",
    "motorway_link",
    "trunk",
    "trunk_link",
    "busway",
    "bus_guideway",
    "raceway",
    "construction",
    "proposed",
    "abandoned",
];

/// `access` and `foot` values that keep walkers out.
const NO_ENTRY: [&str; 2] = ["no", "private"];
/// `foot` values that let walkers in whatever `access` says.
const FOOT_ALLOWED: [&str; 3] = ["yes", "designated", "permissive"];

/// Walkable ways that are not drawn as context: pavements next to a street
/// that is drawn already, crossings, and indoor links.
const NOT_DRAWN_FOOTWAYS: [&str; 2] = ["sidewalk", "crossing"];
const NOT_DRAWN_HIGHWAYS: [&str; 3] = ["elevator", "corridor", "platform"];

/// Tags as Overpass returns them.
pub type Tags = BTreeMap<String, String>;

/// The layer and group of a business or facility, or `None` for anything else.
pub fn place_group(tags: &Tags) -> Option<(Layer, &'static str)> {
    let value = |key: &str| tags.get(key).map(String::as_str);

    if let Some(amenity) = value("amenity") {
        if amenity == PARKING_AMENITY {
            return Some((Layer::Facility, "parking"));
        }
        let found = AMENITIES.iter().find(|(kept, _, _)| *kept == amenity);
        if let Some(&(_, layer, group)) = found {
            return Some((layer, group));
        }
    }
    if let Some(tourism) = value("tourism") {
        let found = TOURISM.iter().find(|(kept, _, _)| *kept == tourism);
        if let Some(&(_, layer, group)) = found {
            return Some((layer, group));
        }
    }
    if value("leisure").is_some_and(|leisure| LEISURE.contains(&leisure)) {
        return Some((Layer::Facility, "leisure"));
    }
    if value("shop").is_some_and(|shop| shop != "vacant" && shop != "no") {
        return Some((Layer::Business, "shop"));
    }
    match value("office") {
        Some("government" | "diplomatic") => return Some((Layer::Facility, "civic")),
        Some(_) => return Some((Layer::Business, "office")),
        None => {}
    }
    if value("craft").is_some() {
        return Some((Layer::Business, "service"));
    }

    None
}

/// The tag value a place was kept for, untranslated: "restaurant", "clothes".
pub fn place_kind(tags: &Tags) -> Option<String> {
    ["amenity", "tourism", "leisure", "shop", "office", "craft"]
        .iter()
        .find_map(|key| tags.get(*key).cloned())
}

/// True for a node that marks where people board: a platform, a stop or a station.
pub fn is_stop(tags: &Tags) -> bool {
    let value = |key: &str| tags.get(key).map(String::as_str);
    matches!(
        value("public_transport"),
        Some("platform" | "stop_position" | "station")
    ) || value("highway") == Some("bus_stop")
        || matches!(value("railway"), Some("tram_stop" | "station" | "halt"))
}

/// True for the node a vehicle stops at on the track or road. It carries the
/// lines but is not where people wait, so it is not drawn on its own.
pub fn is_stop_position(tags: &Tags) -> bool {
    tags.get("public_transport").map(String::as_str) == Some("stop_position")
}

/// The transit group a stop's own tags point to, for stops no route lists.
pub fn stop_mode(tags: &Tags) -> &'static str {
    let value = |key: &str| tags.get(key).map(String::as_str);
    let is_yes = |key: &str| value(key) == Some("yes");
    if value("station") == Some("subway") || is_yes("subway") {
        "metro"
    } else if matches!(value("railway"), Some("station" | "halt")) || is_yes("train") {
        "train"
    } else if value("railway") == Some("tram_stop") || is_yes("tram") {
        "tram"
    } else if is_yes("ferry") {
        "ferry"
    } else {
        "bus"
    }
}

/// The transit group of a route relation's `route` value, or `None` for
/// routes that are not public transport.
pub fn route_mode(route: &str) -> Option<&'static str> {
    match route {
        "train" => Some("train"),
        "subway" => Some("metro"),
        "tram" | "light_rail" => Some("tram"),
        "bus" | "trolleybus" => Some("bus"),
        "ferry" => Some("ferry"),
        _ => None,
    }
}

/// Every `route` value `route_mode` knows, for the query.
pub const ROUTES: [&str; 7] = [
    "train",
    "subway",
    "tram",
    "light_rail",
    "bus",
    "trolleybus",
    "ferry",
];

/// True for a way a person may walk along.
pub fn is_walkable(tags: &Tags) -> bool {
    let value = |key: &str| tags.get(key).map(String::as_str);
    let Some(highway) = value("highway") else {
        return false;
    };
    if NOT_WALKABLE.contains(&highway) || value("area") == Some("yes") {
        return false;
    }
    let foot = value("foot");
    if foot.is_some_and(|foot| NO_ENTRY.contains(&foot)) {
        return false;
    }
    let is_foot_allowed = foot.is_some_and(|foot| FOOT_ALLOWED.contains(&foot));
    let is_closed = value("access").is_some_and(|access| NO_ENTRY.contains(&access));

    is_foot_allowed || !is_closed
}

/// True for a walkable way worth drawing as context around the site.
pub fn is_drawn(tags: &Tags) -> bool {
    let value = |key: &str| tags.get(key).map(String::as_str);
    let highway = value("highway").unwrap_or_default();
    let footway = value("footway").unwrap_or_default();

    !NOT_DRAWN_HIGHWAYS.contains(&highway) && !NOT_DRAWN_FOOTWAYS.contains(&footway)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(pairs: &[(&str, &str)]) -> Tags {
        pairs
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    #[test]
    fn a_restaurant_is_a_food_and_drink_business() {
        let restaurant = tags(&[("amenity", "restaurant"), ("name", "Moeders")]);
        assert_eq!(
            place_group(&restaurant),
            Some((Layer::Business, "food_drink"))
        );
    }

    #[test]
    fn a_cafe_that_is_also_tagged_a_shop_counts_once_as_food_and_drink() {
        let cafe = tags(&[("amenity", "cafe"), ("shop", "coffee")]);
        assert_eq!(place_group(&cafe), Some((Layer::Business, "food_drink")));
    }

    #[test]
    fn street_furniture_and_vacant_shops_are_not_places() {
        assert_eq!(place_group(&tags(&[("amenity", "bench")])), None);
        assert_eq!(
            place_group(&tags(&[("amenity", "bicycle_parking")])),
            Some((Layer::Facility, "bicycle_parking"))
        );
        assert_eq!(place_group(&tags(&[("amenity", "waste_basket")])), None);
        assert_eq!(place_group(&tags(&[("shop", "vacant")])), None);
    }

    #[test]
    fn museums_and_town_halls_are_facilities() {
        let museum = tags(&[("tourism", "museum")]);
        let town_hall = tags(&[("amenity", "townhall")]);
        assert_eq!(place_group(&museum), Some((Layer::Facility, "culture")));
        assert_eq!(place_group(&town_hall), Some((Layer::Facility, "civic")));
    }

    #[test]
    fn every_group_in_the_tables_is_a_listed_group() {
        let groups: Vec<&str> = BUSINESS_GROUPS
            .iter()
            .chain(&FACILITY_GROUPS)
            .map(|(group, _)| *group)
            .collect();
        for (_, _, group) in AMENITIES.iter().chain(&TOURISM) {
            assert!(groups.contains(group), "{group}");
        }
    }

    #[test]
    fn motorways_private_ways_and_no_foot_ways_cannot_be_walked() {
        assert!(!is_walkable(&tags(&[("highway", "motorway")])));
        assert!(!is_walkable(&tags(&[
            ("highway", "service"),
            ("access", "private")
        ])));
        assert!(!is_walkable(&tags(&[
            ("highway", "cycleway"),
            ("foot", "no")
        ])));
        assert!(is_walkable(&tags(&[
            ("highway", "service"),
            ("access", "private"),
            ("foot", "yes")
        ])));
        assert!(is_walkable(&tags(&[("highway", "footway")])));
    }

    #[test]
    fn a_tram_stop_is_a_tram_stop_and_an_unknown_stop_is_a_bus_stop() {
        assert_eq!(stop_mode(&tags(&[("railway", "tram_stop")])), "tram");
        assert_eq!(stop_mode(&tags(&[("railway", "station")])), "train");
        assert_eq!(
            stop_mode(&tags(&[("railway", "station"), ("station", "subway")])),
            "metro"
        );
        assert_eq!(stop_mode(&tags(&[("public_transport", "platform")])), "bus");
    }
}
