# Fixture corpus: `/liveclientdata/allgamedata` payloads

Synthetic-but-schema-faithful recordings of the Live Client Data API
`allgamedata` response shape, grounded in the field inventory documented in
`openspec/changes/add-live-match-dashboard/exploration.md` §3.1 and the
reference models of the `lol-game-client-api` crate (v0.1.x). JSON cannot
carry comments, so provenance for each file lives here:

| File | Purpose | Notes |
|---|---|---|
| `full.json` | Complete capture: 10 players fully populated, local player (`Ahri`, summoner `MidMage`) with `currentGold` + statistics block, 9 mixed events incl. stolen Chemtech DragonKill | Every field asserted by `tests/model/live_data_tests.rs` |
| `partial_player.json` | Identical to `full.json` except CHAOS BOTTOM `Kai'Sa`: `items` and `respawnTimer` keys omitted entirely; `scores` reduced to `{kills, deaths}` (assists/creepScore/wardScore omitted) | Exercises absent-vs-present tolerance (poller:R4/S2) |
| `empty_events.json` | Identical to `full.json` except top-level `events` is an empty array | Exercises empty event list → ticker empty state (poller:R5, ui:R5/S2) |
| `malformed.json` | Deliberately truncated JSON | Must yield a parse error, never a panic (poller:R4/S3) |

Conventions mirroring real captures:

- `team` values are `ORDER` / `CHAOS`; team-recipient event fields use
  capitalized form `Order` / `Chaos`.
- Summoner spell display names carry internal ids (`SummonerFlash`,
  `SummonerDot` = Ignite), as the live client does.
- Dead players expose positive `respawnTimer`; living players expose `0.0`.
  Absence of the key (partial fixture) means unknown — never fabricated.
- Values were authored for this repository on 2026-08-25; no live game was
  recorded. Numeric values are plausible but fictional.
