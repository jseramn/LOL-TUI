# Live Client Poller Specification

## Purpose

Keyless HTTP client for the League of Legends Live Client Data API served by the local game client at `https://127.0.0.1:2999` during the user's own live game. It polls strictly non-overlapping, detects game lifecycle from server availability, and deserializes responses into normalized metrics snapshots consumed by the dashboard UI. All behavior MUST be verifiable offline against recorded payload fixtures; a running game is never required for development or testing.

## Requirements

### Requirement: Loopback-only TLS client trusting the Riot certificate

The poller SHALL exchange data only with `https://127.0.0.1:2999` endpoints (`/liveclientdata/allgamedata`, `/playerlist`, `/eventdata`, `/gamestats`). TLS SHALL validate against the embedded Riot Games root certificate. It MAY offer a localhost-only insecure-TLS fallback that is inactive unless explicitly enabled. It MUST NOT send requests to any non-loopback host.

#### Scenario: Poll succeeds with trusted certificate

- GIVEN a mock server on 127.0.0.1:2999 presenting a certificate chaining to the embedded Riot root
- WHEN the poller fetches `/liveclientdata/allgamedata`
- THEN the request succeeds and the response body is returned

#### Scenario: Untrusted certificate is rejected

- GIVEN a mock server presenting a certificate not chaining to the Riot root, insecure fallback disabled
- WHEN the poller fetches any endpoint
- THEN the attempt fails with an explicit trust error and no body is accepted

#### Scenario: Non-loopback target refused without network traffic

- GIVEN a configured base URL whose host is not 127.0.0.1
- WHEN the poller is constructed
- THEN construction fails and no packet is sent

### Requirement: Strictly non-overlapping poll scheduling

The poller SHALL repeat polls at a fixed cadence within 250–1000 ms (default 1000 ms) and SHALL schedule the next poll only after the previous resolves. Out-of-range cadence values SHALL be clamped into the allowed range.

#### Scenario: Steady cadence under fast responses

- GIVEN cadence 250 ms and responses resolving in under 250 ms
- WHEN several cycles elapse
- THEN polls fire at most once per cadence window and never concurrently

#### Scenario: Slow response does not overlap

- GIVEN cadence 250 ms and a mocked response resolving after 900 ms
- WHEN the cycle completes
- THEN exactly one request was in flight and the next starts only after its resolve

### Requirement: Game-lifecycle detection

The poller SHALL classify connection-refused / port-not-bound as NOT_IN_GAME and a successful response as IN_GAME, emitting a lifecycle transition on each change. Failures other than not-bound refusal (timeout, TLS, malformed body) MUST NOT be classified as game end.

#### Scenario: Idle to live on first contact

- GIVEN the server refuses connections
- WHEN the server begins accepting and answering requests
- THEN a NOT_IN_GAME → IN_GAME transition is emitted

#### Scenario: Mid-game disconnect and reconnect

- GIVEN IN_GAME state
- WHEN the server stops accepting connections, then accepts again
- THEN an IN_GAME → NOT_IN_GAME transition followed by a NOT_IN_GAME → IN_GAME transition is emitted, without a fatal error

#### Scenario: Transient error keeps game state

- GIVEN IN_GAME state
- WHEN one poll times out or returns a malformed body
- THEN lifecycle remains IN_GAME, the failure is reported, and polling continues

### Requirement: Normalized metrics snapshot deserialization

The poller SHALL deserialize payloads into a normalized snapshot covering, per player: champion, team, position, level, kills/deaths/assists, creep score, items, summoner spells (with cooldowns as exposed), isDead, and respawnTimer as exposed; for the local player additionally currentGold and exposed stat detail; plus game stats (gameTime, gameMode, mapName) and the event list. Absent, null, or omitted fields SHALL be represented as absent — never defaulted to fabricated values. A malformed payload MUST produce a reported parse error, preserve the last good snapshot, and continue polling.

#### Scenario: Recorded full payload parses completely

- GIVEN a recorded `/allgamedata` fixture with all 10 players fully populated
- WHEN it is deserialized
- THEN every listed field of every player is present with the fixture's values

#### Scenario: Partial player data degrades gracefully

- GIVEN a fixture where one player omits items, respawnTimer, and parts of scores
- WHEN it is deserialized
- THEN the snapshot builds successfully with those fields marked absent and all other players intact

#### Scenario: Malformed JSON is rejected safely

- GIVEN a fixture containing syntactically invalid JSON
- WHEN it is processed
- THEN a parse error is reported, the previous snapshot is retained, and no panic occurs

### Requirement: Fixture-driven offline testability

All poller behavior SHALL be exercisable by the project test suite using recorded fixtures under `tests/fixtures/`, with no reachable port 2999 and no network access.

#### Scenario: Suite runs fully offline

- GIVEN the canonical env-prefixed `cargo test -j 1` invocation
- WHEN the suite executes on a machine with no LoL client running
- THEN all poller tests pass using only bundled fixtures
