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

/// Tunnel / remote poll deadline. Cloudflare/Tailscale hops need more
/// than the 800 ms loopback window.
pub const REMOTE_FETCH_TIMEOUT: Duration = Duration::from_millis(3000);

/// Environment variable that opts the binary into a tunneled live URL
/// (same as `--live-url`). Empty / unset keeps loopback-only behavior.
pub const LIVE_URL_ENV: &str = "TUI_LOL_LIVE_URL";

/// Vendored Riot Games root certificate (see provenance header inside the
/// file). This is the ONLY trust anchor: built-in system stores are disabled
/// so a compromised host store can never widen trust (poller:R1).
const RIOT_ROOT_PEM: &[u8] = include_bytes!("../../assets/riotgames.pem");

/// Where the client sends `/liveclientdata/*` requests.
#[derive(Debug, Clone)]
enum Target {
    /// Spec-pinned local game client (`https://127.0.0.1:2999`).
    Loopback { port: u16 },
    /// Opt-in tunnel or replay base, e.g. `https://….trycloudflare.com`.
    /// Trailing slashes are stripped; paths are appended as-is.
    Remote { base: String },
}

/// Blocking HTTPS client. The default constructor is loopback-only and
/// pinned to the Riot root; [`ApiClient::remote`] is the explicit cloud/dev
/// override that trusts the public web PKI (Cloudflare/Tailscale certs).
#[derive(Debug, Clone)]
pub struct ApiClient {
    host: String,
    port: u16,
    target: Target,
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
    pub fn with_fetch_timeout(
        host: &str,
        port: u16,
        timeout: Duration,
    ) -> Result<Self, BuildError> {
        if host != LOOPBACK_HOST {
            return Err(BuildError::NonLoopbackHost {
                host: host.to_owned(),
            });
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
        Ok(Self {
            host: host.to_owned(),
            port,
            target: Target::Loopback { port },
            http,
        })
    }

    /// Opt-in client for a tunneled or replay Live Client Data base URL.
    ///
    /// Default [`ApiClient::new`] stays loopback-only (poller spec R1).
    /// This constructor is the cloud-dev seam: `--live-url` /
    /// [`LIVE_URL_ENV`]. TLS uses the public web PKI so Cloudflare/Tailscale
    /// certificates validate; HTTP bases skip TLS entirely.
    ///
    /// # Errors
    /// [`BuildError::InvalidLiveUrl`] when the value is not `http`/`https`
    /// or has no host.
    pub fn remote(base: &str) -> Result<Self, BuildError> {
        Self::remote_with_timeout(base, REMOTE_FETCH_TIMEOUT)
    }

    /// Like [`ApiClient::remote`] with an explicit per-request deadline.
    pub fn remote_with_timeout(base: &str, timeout: Duration) -> Result<Self, BuildError> {
        let parsed =
            reqwest::Url::parse(base.trim()).map_err(|err| BuildError::InvalidLiveUrl {
                detail: err.to_string(),
            })?;
        let scheme = parsed.scheme();
        if scheme != "https" && scheme != "http" {
            return Err(BuildError::InvalidLiveUrl {
                detail: format!("unsupported scheme {scheme:?} (need http or https)"),
            });
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| BuildError::InvalidLiveUrl {
                detail: "URL has no host".into(),
            })?
            .to_owned();
        let port = parsed.port_or_known_default().unwrap_or(443);
        let mut http = reqwest::blocking::Client::builder().timeout(timeout);
        if scheme == "https" {
            // Public CA store: trycloudflare / Tailscale Funnel certs.
            http = http.tls_built_in_root_certs(true);
        }
        let http = http
            .build()
            .expect("reqwest client builder cannot fail with these options");
        let base = base.trim().trim_end_matches('/').to_owned();
        Ok(Self {
            host,
            port,
            target: Target::Remote { base },
            http,
        })
    }

    /// Loopback when `base` is empty/absent; [`ApiClient::remote`] otherwise.
    ///
    /// # Errors
    /// Propagates [`BuildError`] from the chosen constructor.
    pub fn from_optional_base(base: Option<&str>) -> Result<Self, BuildError> {
        match base.map(str::trim).filter(|value| !value.is_empty()) {
            None => Self::new(LOOPBACK_HOST, LIVE_CLIENT_PORT),
            Some(url) => Self::remote(url),
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    /// True when this client talks to a tunneled/remote base rather than
    /// the spec-pinned loopback game client.
    pub fn is_remote(&self) -> bool {
        matches!(self.target, Target::Remote { .. })
    }

    /// Builds the request URL for an endpoint path.
    pub fn url(&self, path: &str) -> String {
        match &self.target {
            Target::Loopback { port } => format!("https://{}:{port}{path}", self.host),
            Target::Remote { base } => format!("{base}{path}"),
        }
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
        response
            .text()
            .map_err(|_| PollError::Transient(TransientReason::Http))
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
