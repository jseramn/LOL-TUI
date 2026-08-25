//! Poller error taxonomy (design table "Error Taxonomy ↔ Spec Scenarios").
//!
//! Two axes, never mixed:
//! - [`BuildError`]: fatal at construction (non-loopback host), before any
//!   packet exists.
//! - [`PollError`]: per-fetch outcome. [`PollError::NotBound`] maps to the
//!   lifecycle `NOT_IN_GAME`; [`PollError::Transient`] maps to
//!   `IN_GAME · Degraded(reason)` and must NEVER end the game lifecycle
//!   (spec R3).

use thiserror::Error;

/// Construction-time failures. Raised before any network object exists.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum BuildError {
    /// Host other than the literal `127.0.0.1` was configured.
    #[error("non-loopback host refused: {host:?} (only 127.0.0.1 is allowed)")]
    NonLoopbackHost { host: String },
}

/// Reason attached to a [`PollError::Transient`] classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TransientReason {
    /// Request exceeded its deadline.
    #[error("timeout")]
    Timeout,
    /// TLS handshake or certificate-trust failure.
    #[error("tls/trust failure")]
    Tls,
    /// HTTP status not usable (>= 500 or otherwise non-success).
    #[error("http status")]
    Http,
    /// Body arrived but was not valid `/allgamedata` JSON.
    #[error("malformed body")]
    Parse,
}

/// Classified outcome of a single fetch attempt.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PollError {
    /// Connection refused / port not bound ⇒ lifecycle `NOT_IN_GAME`.
    #[error("live client port not bound ({0})")]
    NotBound(String),
    /// Timeout / TLS / HTTP / malformed ⇒ stays `IN_GAME · Degraded`.
    #[error("transient failure: {0}")]
    Transient(TransientReason),
}
