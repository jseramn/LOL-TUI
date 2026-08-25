//! Loopback-only TLS client acceptance criteria
//! (spec `live-client-poller` R1: loopback-only TLS trusting the Riot cert).
//!
//! RED phase Unit 2 tasks 2.1–2.4: these tests were authored BEFORE any
//! `src/api` implementation existed (strict-TDD ordering).

use tui_lol::api::client::{ApiClient, LIVE_CLIENT_PORT, LOOPBACK_HOST};
use tui_lol::api::error::BuildError;

/// poller:R1/S3 — a configured base URL whose host is not 127.0.0.1 fails
/// construction, before any packet could be sent. The literal-host rule is
/// deliberate: the spec pins exchange to `https://127.0.0.1:2999` only, so
/// even other loopback spellings (`localhost`, `::1`) are refused.
#[test]
fn non_loopback_hosts_are_refused_at_construction() {
    for host in ["example.com", "localhost", "::1", "192.168.1.10", "0.0.0.0"] {
        let err = ApiClient::new(host, LIVE_CLIENT_PORT)
            .expect_err("non-loopback host must be refused at construction");
        assert!(
            matches!(err, BuildError::NonLoopbackHost { .. }),
            "host {host:?}: expected NonLoopbackHost, got {err:?}"
        );
    }
}

/// Companion boundary: the exact spec-pinned spelling constructs cleanly.
#[test]
fn loopback_host_constructs_client() {
    let client = ApiClient::new(LOOPBACK_HOST, LIVE_CLIENT_PORT).expect("loopback must construct");
    assert_eq!(client.host(), LOOPBACK_HOST);
    assert_eq!(client.port(), LIVE_CLIENT_PORT);
}
