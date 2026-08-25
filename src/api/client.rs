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

use crate::api::error::BuildError;
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

/// Whole-request deadline. Below the maximum cadence so a hung request still
/// leaves room for lifecycle reporting inside one cycle.
const FETCH_TIMEOUT: Duration = Duration::from_millis(800);

/// Blocking HTTPS client pinned to the Riot root certificate.
#[derive(Debug, Clone)]
pub struct ApiClient {
    host: String,
    port: u16,
    http: reqwest::blocking::Client,
}

impl ApiClient {
    /// Constructs the client after validating the loopback guard.
    ///
    /// # Errors
    /// [`BuildError::NonLoopbackHost`] when `host` is not the literal
    /// `127.0.0.1`. No connection state is created on refusal.
    pub fn new(host: &str, port: u16) -> Result<Self, BuildError> {
        if host != LOOPBACK_HOST {
            return Err(BuildError::NonLoopbackHost { host: host.to_owned() });
        }
        let http = reqwest::blocking::Client::builder()
            .timeout(FETCH_TIMEOUT)
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
}
