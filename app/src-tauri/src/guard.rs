//! Every command runs core code through `calm`, so an error, or a panic in the
//! core, reaches the window as one short sentence instead of a crashed command.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs `work` and turns a panic into an error message.
pub fn calm<T>(work: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    match catch_unwind(AssertUnwindSafe(work)) {
        Ok(result) => result,
        Err(payload) => Err(panic_message(payload.as_ref())),
    }
}

/// Runs `work` on a worker thread, so rendering and fetching never block the window.
pub async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || calm(work))
        .await
        .map_err(|error| format!("A background task stopped unexpectedly: {error}"))?
}

/// Core errors already read as sentences ("could not reach PDOK: timeout").
pub fn text(error: openarchi_core::Error) -> String {
    let message = error.to_string();
    let mut letters = message.chars();
    match letters.next() {
        Some(first) => first.to_uppercase().chain(letters).collect(),
        None => message,
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    let message = payload
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| payload.downcast_ref::<&str>().map(|text| text.to_string()))
        .unwrap_or_default();

    format!("Something went wrong inside openarchi: {message}")
}
