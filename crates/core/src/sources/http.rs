//! One blocking HTTP client for every data service, with timeouts and one retry.

use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use ureq::Agent;

use crate::{Error, Result};

/// A whole response, including a slow 2 km elevation tile, must arrive within this.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// PDOK hiccups are usually gone a moment later, so one retry after a short pause.
const RETRY_DELAY: Duration = Duration::from_secs(2);
/// ureq stops at 10 MB by default; a 2 km elevation tile is about 20 MB.
const MAX_BODY_BYTES: u64 = 256 * 1024 * 1024;

/// The response body of `url` with `query` appended. `service` names the
/// service in error messages: "PDOK BGT".
pub fn get(service: &str, url: &str, query: &[(&str, &str)]) -> Result<Vec<u8>> {
    match get_once(service, url, query) {
        Ok(body) => Ok(body),
        Err(_) => {
            thread::sleep(RETRY_DELAY);
            get_once(service, url, query)
        }
    }
}

fn get_once(service: &str, url: &str, query: &[(&str, &str)]) -> Result<Vec<u8>> {
    let network_error = |error: ureq::Error| Error::Network {
        service: service.to_string(),
        detail: error.to_string(),
    };

    let mut response = agent()
        .get(url)
        .query_pairs(query.iter().copied())
        .call()
        .map_err(network_error)?;

    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_BYTES)
        .read_to_vec()
        .map_err(network_error)
}

// Shared so connections to PDOK are reused across pages and sources.
fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .user_agent(concat!("openarchi/", env!("CARGO_PKG_VERSION")))
            .build()
            .into()
    })
}
