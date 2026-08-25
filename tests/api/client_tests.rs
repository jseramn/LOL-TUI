//! Loopback-only TLS client acceptance criteria
//! (spec `live-client-poller` R1: loopback-only TLS trusting the Riot cert).
//!
//! RED phase Unit 2 tasks 2.1–2.4: these tests were authored BEFORE any
//! `src/api` implementation existed (strict-TDD ordering).

use rcgen::{generate_simple_self_signed, CertifiedKey};
use std::net::TcpListener;
use tui_lol::api::client::{ApiClient, ENDPOINT_ALL_GAME_DATA, LIVE_CLIENT_PORT, LOOPBACK_HOST};
use tui_lol::api::error::{BuildError, PollError, TransientReason};

/// poller:R1/S3 — a configured base URL whose host is not 127.0.0.1 fails
/// construction, before any packet could be sent. The literal-host rule is
/// deliberate: the spec pins exchange to `https://127.0.0.1:2999` only, so
/// even other loopback spellings (`localhost`, `::1`) are refused.
#[test]
fn non_loopback_hosts_are_refused_at_construction() {
    for host in ["example.com", "localhost", "::1", "192.168.1.10", "0.0.0.0"] {
        let err =
            ApiClient::new(host, LIVE_CLIENT_PORT).expect_err("non-loopback host must be refused");
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

// ---------------------------------------------------------------------------
// Trust store (task 2.3) — poller:R1/S2
// ---------------------------------------------------------------------------

/// Provenance digest claimed by the header comment inside the vendored pem.
const RIOT_PEM_SHA256: &str =
    "da884275737f024b33c93ae5d28bdb002768a3cb73752ab40254a32218193521";

/// The provenance SHA-256 covers the certificate artifact exactly as
/// published: bytes from the BEGIN CERTIFICATE marker onward, CRLF line
/// endings preserved. The `#` provenance header prepended for humans is NOT
/// part of the digested material.
fn pem_body(pem: &[u8]) -> &[u8] {
    const MARKER: &[u8] = b"-----BEGIN CERTIFICATE-----";
    let at = pem
        .windows(MARKER.len())
        .position(|w| w == MARKER)
        .expect("vendored pem must contain a CERTIFICATE block");
    &pem[at..]
}

#[test]
fn embedded_pem_digest_matches_provenance_header() {
    use sha2::Digest;
    let pem = include_bytes!("../../assets/riotgames.pem");
    let digest = sha2::Sha256::digest(pem_body(pem));
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(hex, RIOT_PEM_SHA256);
}

/// Exactly one certificate is embedded: combined with the digest match this
/// proves the trust anchor is the Riot root and nothing else rode along.
#[test]
fn embedded_trust_material_is_exactly_one_certificate() {
    let pem = include_bytes!("../../assets/riotgames.pem");
    const MARKER: &[u8] = b"-----BEGIN CERTIFICATE-----";
    let count = pem.windows(MARKER.len()).filter(|w| *w == MARKER).count();
    assert_eq!(count, 1, "trust store source must hold a single root");
}

/// poller:R1/S2 — an rcgen-generated chain that does NOT chain to the
/// embedded Riot root is rejected with an explicit TLS/trust classification.
/// Honest scope: positive acceptance of a Riot-chained cert is impossible
/// offline (Riot's private key is unavailable) and stays manual-checklist
/// verified (task 6.3).
#[test]
fn untrusted_certificate_chain_is_rejected_as_transient_tls() {
    use std::sync::Arc;

    let CertifiedKey { cert, signing_key } =
        generate_simple_self_signed(vec!["127.0.0.1".into()]).expect("keygen works");
    let priv_key = rustls::pki_types::PrivateKeyDer::Pkcs8(signing_key.serialize_der().into());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("default protocol versions available")
        .with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()], priv_key)
        .expect("server config accepts generated pair");

    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral bind works");
    let port = listener.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        if let Ok((mut sock, _)) = listener.accept() {
            let mut conn = match rustls::ServerConnection::new(Arc::new(config)) {
                Ok(c) => c,
                Err(_) => return,
            };
            // Drive the handshake far enough for the client to see our
            // untrusted certificate; the outcome itself does not matter.
            let _ = conn.complete_io(&mut sock);
        }
    });

    let client = ApiClient::new(LOOPBACK_HOST, port).expect("loopback constructs");
    let err = client
        .fetch(ENDPOINT_ALL_GAME_DATA)
        .expect_err("untrusted chain must not yield a body");
    assert_eq!(
        err,
        PollError::Transient(TransientReason::Tls),
        "TLS rejection must classify as Transient(Tls), got {err:?}"
    );
    server.join().expect("tls server thread joins");
}

// ---------------------------------------------------------------------------
// Transport classification (tasks 2.2/2.4) — design error-taxonomy rows
// ---------------------------------------------------------------------------

/// Connection refused on a just-released loopback port classifies as NotBound
/// — the ONLY failure class allowed to end the game lifecycle (R3/S1,S2).
///
/// Uses an extended deadline: this development machine's security layer
/// delays loopback RSTs by ~2 s (first SYN silently dropped), so the
/// production 800 ms default would legitimately time out first. The test
/// gives the OS room to refuse and asserts the genuine refusal path.
#[test]
fn refused_connection_classifies_as_not_bound() {
    use std::time::Duration;

    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral bind works");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // release: nothing is bound anymore

    let client =
        ApiClient::with_fetch_timeout(LOOPBACK_HOST, port, Duration::from_secs(6))
            .expect("loopback constructs");
    let err = client.fetch(ENDPOINT_ALL_GAME_DATA);
    match err {
        Err(PollError::NotBound(detail)) => assert!(!detail.is_empty()),
        other => panic!("expected NotBound, got {other:?}"),
    }
}

/// A bound-but-silent listener holds the connection open without answering,
/// so the request deadline expires: classified Transient(Timeout), never as
/// game end.
#[test]
fn silent_listener_times_out_as_transient_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("ephemeral bind works");
    let port = listener.local_addr().unwrap().port();
    // Intentionally never accepted.

    let client = ApiClient::new(LOOPBACK_HOST, port).expect("loopback constructs");
    let err = client
        .fetch(ENDPOINT_ALL_GAME_DATA)
        .expect_err("silent server must time out");
    drop(listener);
    assert_eq!(err, PollError::Transient(TransientReason::Timeout));
}

/// Status policy (pure seam in `api::source`): every 2xx is usable.
#[test]
fn success_statuses_pass_status_policy() {
    use reqwest::StatusCode;
    use tui_lol::api::source::ensure_usable_status;

    for code in [200u16, 201, 202, 204] {
        let status = StatusCode::from_u16(code).expect("valid status");
        ensure_usable_status(status).expect("{code} must be usable");
    }
}

/// Odd statuses — anything non-success, including redirects that survived and
/// >=500 — are Transient(Http) per the taxonomy table.
#[test]
fn odd_statuses_are_transient_http() {
    use reqwest::StatusCode;
    use tui_lol::api::source::ensure_usable_status;

    for code in [301u16, 400, 404, 500, 502, 503] {
        let status = StatusCode::from_u16(code).expect("valid status");
        let err = ensure_usable_status(status).expect_err("non-success must be rejected");
        assert_eq!(
            err,
            PollError::Transient(TransientReason::Http),
            "status {code}: expected Transient(Http), got {err:?}"
        );
    }
}
