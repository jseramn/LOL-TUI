//! Loopback-only TLS client for the Live Client Data API served by the game
//! client at `https://127.0.0.1:2999` (spec `live-client-poller` R1).
//!
//! Guard rails enforced HERE, before any packet exists:
//! - Host must be the literal `127.0.0.1` ([`LOOPBACK_HOST`]); every other
//!   spelling is refused at construction (poller:R1/S3).
//! - TLS trusts ONLY the vendored Riot root (`assets/riotgames.pem`);
//!   system/built-in root stores are disabled (poller:R1/S2).
//!
//! D1/W2: the four spec-mandated endpoint paths exist as constants, but the
//! only fetch surface is the aggregate `/liveclientdata/allgamedata` — no
//! speculative per-endpoint fetchers.

use crate::api::error::{BuildError, PollError, TransientReason};
use crate::api::source::ensure_usable_status;
use std::error::Error as StdError;
use std::time::Duration;

/// The only host this client may ever talk to (spec-pinned spelling).
pub const LOOPBACK_HOST: &str = "127.0.0.1";

/// Live Client Data API port.
pub const LIVE_CLIENT_PORT: u16 = 2999;

/// Aggregate payload carrying playerlist + events + gamestats (D1).
pub const ENDPOINT_ALL_GAME_DATA: &str = "/liveclientdata/allgamedata";

/// Allowed by spec R1; intentionally unfetched (W2 — reserved constant).
pub const ENDPOINT_PLAYERLIST: &str = "/liveclientdata/playerlist";

/// Allowed by spec R1; intentionally unfetched (W2 — reserved constant).
pub const ENDPOINT_EVENTDATA: &str = "/liveclientdata/eventdata";

/// Allowed by spec R1; intentionally unfetched (W2 — reserved constant).
pub const ENDPOINT_GAMESTATS: &str = "/liveclientdata/gamestats";

/// Production fetch deadline. Below the maximum cadence so a hung request
/// still leaves room for lifecycle reporting inside one cycle.
///
/// Note: some hardened machines delay loopback connection-refusal past this
/// deadline (SYN filtering); such hosts report idle as `Transient(Timeout)`
/// instead of `NotBound`. Deployment may tune via [`ApiClient::with_fetch_timeout`].
pub const DEFAULT_FETCH_TIMEOUT: Duration = Duration::from_millis(800);

/// Vendored Riot Games root certificate (see provenance header inside the
/// file). This is the ONLY trust anchor: built-in system stores are disabled
/// so a compromised host store can never widen trust (poller:R1).
const RIOT_ROOT_PEM: &[u8] = include_bytes!("../../assets/riotgames.pem");

/// Blocking HTTPS client pinned to the Riot root certificate.
#[derive(Debug, Clone)]
pub struct ApiClient {
    host: String,
    port: u16,
    http: reqwest::blocking::Client,
}

impl ApiClient {
    /// Constructs the client after validating the loopback guard, with the
    /// production fetch deadline ([`DEFAULT_FETCH_TIMEOUT`]).
    ///
    /// # Errors
    /// [`BuildError::NonLoopbackHost`] when `host` is not the literal
    /// `127.0.0.1`. No connection state is created on refusal.
    pub fn new(host: &str, port: u16) -> Result<Self, BuildError> {
        Self::with_fetch_timeout(host, port, DEFAULT_FETCH_TIMEOUT)
    }

    /// Like [`ApiClient::new`] with an explicit per-request deadline.
    ///
    /// # Errors
    /// [`BuildError::NonLoopbackHost`] when `host` is not the literal
    /// `127.0.0.1`.
    pub fn with_fetch_timeout(host: &str, port: u16, timeout: Duration) -> Result<Self, BuildError> {
        if host != LOOPBACK_HOST {
            return Err(BuildError::NonLoopbackHost { host: host.to_owned() });
        }
        let riot_root = reqwest::tls::Certificate::from_pem(RIOT_ROOT_PEM)
            .expect("vendored riotgames.pem must parse");
        let http = reqwest::blocking::Client::builder()
            .timeout(timeout)
            // Trust ONLY the vendored Riot root (poller:R1/S2).
            .tls_built_in_root_certs(false)
            .add_root_certificate(riot_root)
            .build()
            .expect("reqwest client builder cannot fail with these options");
        Ok(Self { host: host.to_owned(), port, http })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// Builds the exact loopback HTTPS URL for an endpoint path.
    pub fn url(&self, path: &str) -> String {
        format!("https://{}:{}{}", self.host, self.port, path)
    }

    /// Performs one GET against an endpoint path and returns the body.
    ///
    /// Failures are classified per the design taxonomy:
    /// - deadline exceeded ⇒ [`PollError::Transient`]([`TransientReason::Timeout`])
    /// - TLS/trust rejection ⇒ `Transient(Tls)` (never NotBound — spec R3
    ///   forbids classifying TLS failure as game end)
    /// - connect refusal on the loopback port ⇒ [`PollError::NotBound`]
    /// - non-success status / body-read failure ⇒ `Transient(Http)`
    pub fn fetch(&self, path: &str) -> Result<String, PollError> {
        let url = self.url(path);
        let response = self.http.get(&url).send().map_err(classify_transport)?;
        ensure_usable_status(response.status())?;
        response.text().map_err(|_| PollError::Transient(TransientReason::Http))
    }
}

/// Maps a reqwest transport error onto the poller taxonomy.
fn classify_transport(err: reqwest::Error) -> PollError {
    if err.is_timeout() {
        return PollError::Transient(TransientReason::Timeout);
    }
    if error_chain_mentions_tls(&err) {
        return PollError::Transient(TransientReason::Tls);
    }
    if err.is_connect() {
        return PollError::NotBound(err.to_string());
    }
    if err.is_status() {
        return PollError::Transient(TransientReason::Http);
    }
    // Unknown transport oddity: degrade without touching lifecycle state.
    PollError::Transient(TransientReason::Http)
}

/// Walks the reqwest error source chain looking for TLS/trust markers.
///
/// rustls certificate rejections surface through hyper without a dedicated
/// reqwest accessor, so the chain text is inspected. Connection-refused and
/// timeout messages contain none of these markers.
fn error_chain_mentions_tls(err: &reqwest::Error) -> bool {
    const MARKERS: [&str; 5] = ["certificate", "tls", "ssl", "handshake", "alert"];
    let mut current: Option<&dyn StdError> = Some(err);
    while let Some(item) = current {
        let text = item.to_string().to_ascii_lowercase();
        if MARKERS.iter().any(|marker| text.contains(marker)) {
            return true;
        }
        current = item.source();
    }
    false
}
