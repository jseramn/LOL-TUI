//! `HttpSource` trait: fetch seam that keeps poller logic offline-testable
//! (design D5). The production implementation is [`ApiClient`]; tests supply
//! fixture-backed fakes instead of hitting any network.

use crate::api::client::ApiClient;
use crate::api::error::{PollError, TransientReason};
use reqwest::StatusCode;

/// One HTTP GET returning the raw body, classified into the poller taxonomy.
pub trait HttpSource {
    /// # Errors
    /// [`PollError::NotBound`] or [`PollError::Transient`] per taxonomy.
    fn fetch(&self, path: &str) -> Result<String, PollError>;
}

impl HttpSource for ApiClient {
    fn fetch(&self, path: &str) -> Result<String, PollError> {
        ApiClient::fetch(self, path)
    }
}

/// Status policy from the design table row "HTTP >=500 / odd status": only
/// 2xx responses carry usable payloads.
///
/// # Errors
/// [`PollError::Transient`]([`TransientReason::Http`]) for every non-success
/// status.
pub fn ensure_usable_status(status: StatusCode) -> Result<(), PollError> {
    if status.is_success() {
        Ok(())
    } else {
        Err(PollError::Transient(TransientReason::Http))
    }
}
