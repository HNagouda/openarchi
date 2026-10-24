//! One blocking HTTP client for every data service, with timeouts and retries.

use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use ureq::Agent;

use crate::{Error, Result};

/// A whole response, including a slow 2 km elevation tile, must arrive within this.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Pauses before each retry. PDOK hiccups are usually gone a moment later,
/// but the CBS grid kept answering 500 to some tiles past one retry while a whole
/// site was fetched at once, so a failed request gets a second, longer pause.
const RETRY_DELAYS: [Duration; 2] = [Duration::from_secs(2), Duration::from_secs(4)];
/// ureq stops at 10 MB by default; a 2 km elevation tile is about 20 MB.
const MAX_BODY_BYTES: u64 = 256 * 1024 * 1024;

/// The response body of `url` with `query` appended. `service` names the
/// service in error messages: "PDOK BGT".
pub fn get(service: &str, url: &str, query: &[(&str, &str)]) -> Result<Vec<u8>> {
    let mut result = get_once(service, url, query);
    for delay in RETRY_DELAYS {
        if result.is_ok() {
            break;
        }
        thread::sleep(delay);
        result = get_once(service, url, query);
    }

    result
}

/// The response body of `url` with `query` appended, asked once and given up
/// after `timeout`. For quick questions that have an answer to fall back on,
/// where waiting through retries would hold up a whole site.
pub fn get_once_within(
    service: &str,
    url: &str,
    query: &[(&str, &str)],
    timeout: Duration,
) -> Result<Vec<u8>> {
    let mut response = agent()
        .get(url)
        .config()
        .timeout_global(Some(timeout))
        .build()
        .query_pairs(query.iter().copied())
        .call()
        .map_err(|error| network_error(service, error))?;

    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_BYTES)
        .read_to_vec()
        .map_err(|error| network_error(service, error))
}

/// The response body of a form POST to `url`, retried after each pause in
/// `backoff` when the server says it is busy (429, 502, 503 or 504). Any other
/// failure is returned at once. `timeout` replaces the usual request timeout,
/// for services that take long to answer.
pub fn post_form(
    service: &str,
    url: &str,
    form: &[(&str, &str)],
    timeout: Duration,
    backoff: &[Duration],
) -> Result<Vec<u8>> {
    let mut pauses = backoff.iter();
    loop {
        let result = agent()
            .post(url)
            .config()
            .timeout_global(Some(timeout))
            .build()
            .send_form(form.iter().copied());
        let error = match result {
            Ok(mut response) => {
                return response
                    .body_mut()
                    .with_config()
                    .limit(MAX_BODY_BYTES)
                    .read_to_vec()
                    .map_err(|error| network_error(service, error));
            }
            Err(error) => error,
        };
        let is_busy = matches!(error, ureq::Error::StatusCode(429 | 502 | 503 | 504));
        match pauses.next() {
            Some(pause) if is_busy => thread::sleep(*pause),
            _ => return Err(network_error(service, error)),
        }
    }
}

fn network_error(service: &str, error: ureq::Error) -> Error {
    Error::Network {
        service: service.to_string(),
        detail: error.to_string(),
    }
}

fn get_once(service: &str, url: &str, query: &[(&str, &str)]) -> Result<Vec<u8>> {
    let mut response = agent()
        .get(url)
        .query_pairs(query.iter().copied())
        .call()
        .map_err(|error| network_error(service, error))?;

    response
        .body_mut()
        .with_config()
        .limit(MAX_BODY_BYTES)
        .read_to_vec()
        .map_err(|error| network_error(service, error))
}

// Shared so connections to PDOK are reused across pages and sources.
fn agent() -> &'static Agent {
    static AGENT: OnceLock<Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        Agent::config_builder()
            .timeout_global(Some(REQUEST_TIMEOUT))
            .timeout_connect(Some(CONNECT_TIMEOUT))
            // Overpass asks every client for a User-Agent that leads back to it.
            .user_agent(concat!(
                "openarchi/",
                env!("CARGO_PKG_VERSION"),
                " (+",
                env!("CARGO_PKG_REPOSITORY"),
                ")"
            ))
            .build()
            .into()
    })
}
