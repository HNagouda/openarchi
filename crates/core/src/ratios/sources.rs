//! The line under each group's heading that says where its numbers come from.
//!
//! Every group id has its line here, in one place, so a new group without one
//! fails `every_group_names_its_source`. Names come from the sources' credits
//! and dated sources carry the year the scene was fetched in.

use crate::attribution::{dated, short_name};
use crate::scene::{Layer, Scene};
use crate::sources::{ahn, bag, bgt, brk, osm, rivm};
use crate::walk::SPEED_M_S;

/// "Buildings from BAG, the ground from BGT." Empty for a group id it does not know.
pub fn line(group_id: &str, scene: &Scene) -> String {
    let bag = short_name(&bag::credit());
    let bgt = short_name(&bgt::credit());
    let brk = short_name(&brk::credit());
    let ahn = short_name(&ahn::credit());
    let osm = short_name(&osm::credit());
    match group_id {
        "land_use" => format!("Buildings from {bag}, the ground from {bgt}."),
        "transport" => format!("Roads, paths, parking and rail areas from {bgt}."),
        "building_use" => format!("Uses of every unit in the building from {bag}."),
        "floors" | "height" => format!("Roof heights measured in {ahn} over {bag} footprints."),
        "year_built" => format!("Construction years from {bag}."),
        "footprint" => format!("Footprints from {bag}."),
        "coverage" => format!("Parcels from {brk}, buildings from {bag}."),
        "floor_area_ratio" => {
            format!("Parcels from {brk}, buildings from {bag}, floors from {ahn} roof heights.")
        }
        "green" => format!("Planted and natural areas from {bgt}."),
        "water" => format!("Water and banks from {bgt}."),
        "businesses" | "facilities" | "parking" => format!("Places mapped by {osm}."),
        "transit_stops_10_min" => {
            format!("Stops mapped by {osm}, reached over its footpaths at {SPEED_M_S} m/s.")
        }
        "road_noise" => format!(
            "Road traffic noise maps from {}.",
            dated(&rivm::credit(), scene.year_of(Layer::RoadNoise))
        ),
        "dwellings" => {
            format!("Dwellings counted from the units {bag} registers in each building.")
        }
        "floor_area" => format!("Usable floor area of every unit from {bag}."),
        _ => String::new(),
    }
}
