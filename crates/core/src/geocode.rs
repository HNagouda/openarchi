//! Address and place search.

use serde::{Deserialize, Serialize};

use crate::area::LonLat;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Place {
    /// "Westermarkt 20, 1016DK Amsterdam".
    pub label: String,
    /// "adres", "weg", "woonplaats" and so on, as the service names it.
    pub kind: String,
    pub location: LonLat,
}

/// Up to `limit` places matching `query`, best first.
pub fn search(query: &str, limit: usize) -> Result<Vec<Place>> {
    let _ = (query, limit);
    todo!("data layer")
}
