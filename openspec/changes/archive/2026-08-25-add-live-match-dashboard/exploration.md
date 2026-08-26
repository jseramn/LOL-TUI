# Exploration: add-live-match-dashboard

**Change**: `add-live-match-dashboard` · **Phase**: explore · **Date**: 2026-08-25
**Scope**: Research-only investigation of the League of Legends / Riot Games API ecosystem to ground a real-time ratatui "operational intelligence dashboard" (metrics for the user and all 10 players, live panels: KDA, gold, objectives, timers, events).

---

## Executive Summary

1. **True real-time exists in exactly one place**: the **Live Client Data API** (`https://127.0.0.1:2999`) served by the game client **on the user's own machine during their own live game**. It exposes per-second-level KDA, CS, level, items, abilities, runes, respawn timers and objective events for all 10 players — but **gold is available only for the active (local) player**, and there are no positions/minimap coordinates.
2. **There is no sanctioned way to pull live in-game stats of OTHER players' matches.** Spectator-V5 returns only lobby-level metadata (roster, picks, bans, spells, start time). The internal spectator chunk grid (`observer-mode/rest/consumer`, ~3-min delayed) is undocumented and explicitly called "unsupported" by Riot DevRel — treat as out of scope.
3. **Post-game Match-V5 data lands within minutes typically** (no official SLA; a fraction of a percent of matches never upload; retention: matches 2 years, timelines 1 year, list capped at last 1000 games). Timelines provide per-minute gold/XP/CS/items/positions for all 10 players — the richest analytics payload, but only after the game ends.
4. A **personal API key is sufficient** for this product's realistic scope (private, self-use/small-community tool): 20 req/s + 100 req/2min per region, no daily expiry like dev keys. Production keys require website + review (community-reported backlog: **6–9 months** as of early 2026) — do not plan around one.
5. **Compliance boundary matters**: Riot's Game Integrity policy forbids surfacing information *not present in the game client* (their example: enemy ult cooldown tracking). A dashboard displaying what the official Live Client Data API exposes is an accepted pattern (Porofessor et al.), but the app must not derive hidden timers, and must ship the required "not endorsed by Riot Games" boilerplate.

---

## 1. Riot Public API Surface (developer.riotgames.com)

Auth for all web APIs: header `X-Riot-Token: <API_KEY>`. JSON responses; empty values omitted (assume 0/empty/null).

| API | Version | Route type | Key endpoints | Dashboard use |
|---|---|---|---|---|
| **Account-V1** | v1 | Regional | `GET /riot/account/v1/accounts/by-riot-id/{gameName}/{tagLine}` · `/by-puuid/{puuid}` | Resolve user-entered Riot ID → PUUID |
| **Summoner-V4** | v4 | Platform | `GET /lol/summoner/v4/summoners/by-name/{name}` · `/by-puuid/{encryptedPUUID}` · `/by-summoner/{id}` · `/me` | PUUID ↔ summonerId linkage, level, icon |
| **Match-V5** | v5 | Regional | `GET /lol/match/v5/matches/by-puuid/{puuid}/ids?start&count≤100&queue&type&startTime&endTime`<br>`GET /lol/match/v5/matches/{matchId}`<br>`GET /lol/match/v5/matches/{matchId}/timeline` | Enter match ID → full box score; timeline = per-minute gold/XP/CS/items/positions + event stream (the post-game analytics backbone) |
| **Spectator-V5** | v5 | Platform | `GET /lol/spectator/v5/active-games/by-puuid/{encryptedPUUID}`<br>`GET /lol/spectator/v5/featured-games` | Detect a player's live game; roster/picks/bans metadata only |
| **League-V4** | v4 | Platform | `GET /lol/league/v4/entries/by-summoner/{encryptedSummonerId}` · `/entries/{queue}/{tier}/{division}` · challenger/grandmaster/master by queue | Ranked tier/division/LP badges per player |
| **Champion Mastery-V4** | v4 | Platform | `GET /lol/champion-mastery/v4/champion-masteries/by-puuid/{puuid}` · `/by-champion/{championId}` · total-score variants | Champion familiarity panels |

Notes:
- Match IDs are `{PLATFORM}_{gameIdNumeric}`, e.g. `BR1_1234567890`.
- Gotcha (documented): `info.gameDuration` is **seconds** for post-11.20 matches and milliseconds for older ones — detect via presence of `gameEndTimestamp`.
- Timeline structure: `info.frameInterval = 60000`; `frames[].participantFrames` carries `currentGold`, `totalGold`, `cs` (+jungle), `xp`, `level`, `position{x,y}`, damage stats, item snapshots; `frames[].events` carries `CHAMPION_KILL` (with victim damage decomposition), `ELITE_MONSTER_KILLED` (dragon types/baron/herald/grubs via `monsterType`), `BUILDING_KILLED` (turret tier/lane, inhibitor), ward/item/level-up events.
- Known service realities: intermittent 404s for a small fraction of listed matches ("match file not found", wont-fix), occasional long hangs (~2% historically) — build tolerant retries/caching. ([issue #807](https://github.com/RiotGames/developer-relations/issues/807), [#707](https://github.com/RiotGames/developer-relations/issues/707))

### Routing model (LATAM-relevant)

| Route kind | Host | Covers |
|---|---|---|
| Platform | `br1.api.riotgames.com` | Brazil (LAN=LA1, LAS=LA2 also LATAM platforms) |
| Platform | `la1.api.riotgames.com` / `la2.api.riotgames.com` | LAN / LAS |
| Regional | `americas.api.riotgames.com` | NA1, **BR1, LA1, LA2** → used by Match-V5, Account-V1 |

Rule of thumb: *player-profile* endpoints (Summoner, League, Mastery, Spectator) → **platform host**; *account & match* endpoints → **regional host**. For a LATAM user: resolve Riot ID on `americas`, then pick the platform from the returned summoner/platform context (BR1 vs LA1 vs LA2) for profile/live lookups.

Sources: [Routing values](https://developer.riotgames.com/docs/lol), [routing explainer](https://darkintaqt.com/blog/routing), [hextechdocs getting started](https://hextechdocs.dev/getting-started-with-the-riot-games-api/).

---

## 2. Auth Model: Dev vs Personal vs Production

Portal source: [Rate limiting & keys](https://developer.riotgames.com/docs/portal).

| Key tier | Limits (per region) | Expiry / renewal | Requirements |
|---|---|---|---|
| **Development** (auto-granted on portal login) | 20 req / 1 s · 100 req / 2 min | **Deactivates every 24 h**; manual re-reset needed | None |
| **Personal** (register product, no site verification) | Same: 20 req/1 s · 100 req/2 min | No daily reset documented; persists while registered | Product description required; Standard APIs only; **not** for public consumption (incl. open alpha/beta) |
| **Production** (registered + verified product) | Starts 500 req / 10 s · 30 000 req / 10 min (per region); increases by request | Long-lived | Working prototype + owned website with ToS + Privacy Policy; review officially "~weekly, up to 3 weeks"; **community-reported actual backlog 6–9 months** (Jan 2026) ([issue #1126](https://github.com/RiotGames/developer-relations/issues/1126)) |

**Can this ship on a personal dev/personal key?** Yes — comfortably, because the dashboard's heavy traffic is *local* (port 2999 needs no key and has no quota). Web API usage is bursty and small: 1 account lookup + 1 spectator poll cycle + a handful of League/Mastery calls per player refresh, plus one-time immutable match fetches (cacheable forever). At 100 req/2min, polling Spectator-V5 once per 30–60 s per tracked player fits trivially. Caveats from #1126: some portal docs are stale; treat quoted numbers as current-but-unofficial; respect `Retry-After` on 429 or risk suspension.

Key hygiene (policy): never embed the key in a distributed binary; load from env var/local config; HTTPS only.

---

## 3. REAL-TIME Reality Check (critical)

### 3.1 Live Client Data API — `https://127.0.0.1:2999` (own game only)

Official docs: [Game Client APIs](https://developer.riotgames.com/docs/lol#game-client-api). HTTPS with a **self-signed Riot certificate** — either trust Riot's published root cert (`riotgames.pem` linked from that docs page; the Rust crate `lol-game-client-api` bundles exactly this approach via `reqwest::Certificate::from_pem`) or bypass TLS validation for this localhost client only. Local swagger available in-game: `curl -k https://127.0.0.1:2999/swagger/v3/openapi.json`. Server binds only **while a game is running** (connection refused otherwise → natural game-start/end detector).

Endpoints:

| Endpoint | Returns |
|---|---|
| `GET /liveclientdata/allgamedata` | Everything below combined (large; fine at ≤1 Hz) |
| `GET /liveclientdata/activeplayer` | Local player: full stat block (HP/resource, AD/AP, armor/MR, crit, lethality, lifesteal, movespeed…), **`currentGold`**, level, Q/W/E/R+passive with levels, full rune pages |
| `GET /liveclientdata/activeplayername` | Riot ID shim (`riotId`, `riotIdGameName`, `riotIdTagLine`) |
| `GET /liveclientdata/playerlist` | All 10 players: `championName`, `team ORDER/CHAOS`, `position`, `level`, `scores {kills, deaths, assists, creepScore, wardScore}`, `items[]`, `summonerSpells`, partial runes, `isDead`, **`respawnTimer`** |
| `GET /liveclientdata/eventdata` | Full event list (see below) |
| `GET /liveclientdata/gamestats` | `gameTime`, `gameMode`, `mapName/mapNumber/mapTerrain`, game state |

Events emitted (`eventdata.events`, each with `EventID` + `EventTime`): `GameStart`, `GameEnd(Result Win/Loss)`, `MinionsSpawning`, `FirstBrick`, `FirstBlood(Recipient)`, `TurretKilled(Killer,Turret,Assisters)`, `InhibKilled`, `InhibRespawningSoon`, `InhibRespawned`, `DragonKill(DragonType: Elder/Earth/Cloud/Fire/Water/Hextech/Chemtech, Stolen)`, `HeraldKill(Stolen)`, `BaronKill(Stolen)`, `ChampionKill(Killer,Victim,Assisters)`, `Multikill(KillStreak)`, `Ace(AcingTeam)`. Field inventory cross-checked against the `lol-game-client-api` crate models (v0.1.8, July 2026): [model.rs](https://docs.rs/lol-game-client-api/latest/src/lol_game_client_api/model.rs.html).

Update cadence & practical guidance:
- No official polling SLA. Community/event-wrapper practice: **poll every 250–1000 ms, non-overlapping** (dysolix/ingame-api defaults to 1000 ms and schedules next poll only after the previous resolves). `allgamedata` is tens-of-KB; 500 ms is safe on any modern machine.
- **Hard limitation**: other players' **gold is NOT exposed** (only local `currentGold`). Gold-per-enemy panels are impossible live; CS/KDA/level/items/respawn timers are possible for all 10.
- No positions, no cooldown tracking beyond what the client shows, no jungle timer values.
- RiotID migration: endpoints returning `summonerName` now carry a Riot ID shim (`riotId`, `riotIdGameName`, `riotIdTagLine`); parameterized lookups accept `riotId`.

### 3.2 Spectator-V5 (watching others' live games) — metadata only

- Returns `CurrentGameInfo`: participants (champions picked, spells, bot flag), `bannedChampions[]`, `gameStartTime`, `gameMode`, `observers.encryptionKey`, `gameQueueConfigId`. **No live stats whatsoever.**
- Client-side spectating has an anti-ghosting **180 s delay** (SR; 60 s ARAM) — and that path serves the *replay chunks*, which flow through the internal `observer-mode/rest/consumer/getGameMetaData` grid (`delayTime` field). Riot DevRel on that grid: *"this is an unsupported resource"* ([issue #1064](https://github.com/RiotGames/developer-relations/issues/1064)). Building live stats for third-party matches on top of it is fragile and unsanctioned → **out of scope**.
- Reliability history relevant to LA users: stale/previous-game responses on **BR/LAN/LAS** during peak hours ([#477](https://github.com/RiotGames/developer-relations/issues/477)), KR outages ([#941](https://github.com/RiotGames/developer-relations/issues/941)), Arena/customs intermittently 404ing ([#923](https://github.com/RiotGames/developer-relations/issues/923)). Design the panel to degrade gracefully ("no live game detected").

### 3.3 Post-game Match-V5 availability

- No official SLA. Empirically minutes (commonly cited ~1–5 min for match JSON; timeline follows similarly). Failure modes: small fraction of matches permanently 404 despite appearing in history lists (#807); rare regional upload backlogs lasting hours-days have happened (#644, resolved).
- Retention policy (quoted by Riot staff in [#957](https://github.com/RiotGames/developer-relations/issues/957)): **matches kept 2 years, timelines 1 year, per-player list capped at last 1000 games** (customs/practice count toward the cap).
- Implication: "enter your match ID → rich 10-player analytics within a couple of minutes of game end" is a solid promise; "instantly" is not.

---

## 4. LCU API (League Client) — value-add vs deferral

- **Discovery/auth**: `lockfile` in install dir → `LeagueClient:<pid>:<port>:<password>:https`; or read `--app-port=` / `--remoting-auth-token=` from the `LeagueClientUx.exe` command line (`Get-CimInstance Win32_Process` on modern Windows; `wmic` is deprecated). HTTP Basic auth `riot:<password>`, self-signed TLS (same Riot root cert pattern). WebSocket at `wss://riot:<password>@127.0.0.1:<port>` speaking **WAMP 1.0** (subscribe `[5, "OnJsonApiEvent"]`). Sources: [hextechdocs LCU](https://hextechdocs.dev/getting-started-with-the-lcu-api/), [websocket](https://hextechdocs.dev/getting-started-with-the-lcu-websocket/), [lcu.kebs.dev](https://lcu.kebs.dev/).
- **Adds over port 2999**: gameflow-phase push events (None→Lobby→Matchmaking→**ChampSelect**→InProgress→EndOfGame), champ-select draft state, current summoner identity, friends/chat, launching spectator sessions, client control. Rust ecosystem exists (e.g. `league-connect` concepts ported; crates: `lcu`-style wrappers vary in maintenance).
- **Recommendation: defer to a later iteration.** The dashboard's core promise (live metrics during the game) is fully served by port 2999; match-ID entry covers post-game. LCU earns its complexity when we want zero-input UX (auto-detect champ select → preload ranked/masteries of the 10 players before loading screen ends — a genuinely compelling v2). Keep a trait boundary (`LiveSource`) so an LCU provider slots in later.

## 5. Static Data — Data Dragon / CommunityDragon

- **No API key, no rate-limit enforcement concerns** (does not count against app limits).
- Version discovery: `GET https://ddragon.leagueoflegends.com/api/versions.json` → **verified live today: `16.16.1`** is latest (scheme remains `seasonMajor.minor.1`; season 16 = 2026). First array element = current patch.
- Payloads: `https://ddragon.leagueoflegends.com/cdn/{version}/data/en_US/champion.json` (+ per-champion `{champ}.json`), `item.json`, `runesReforged.json`, `summoner.json`; images under `/cdn/{version}/img/...`, splashes `img/champion/splash/`.
- CommunityDragon (`raw.communitydragon.org`) for newest/unreleased assets and localized strings — optional, keyless.
- Cache per patch version on disk; invalidate when `versions.json[0]` changes.

---

## 6. Constraints, ToS & Risks

Policy sources: [Developer API Policy](https://developer.riotgames.com/docs/lol) (General/Game policies), [General policies](https://developer.riotgames.com/policies/general), [Terms](https://developer.riotgames.com/terms).

1. **Game Integrity (hard boundary)**: *"Products must not use or incorporate information not present in the game client that would give players a competitive edge (e.g., automatically or manually allowing tracking enemy ultimate cooldowns)."* → Display only what Live Client Data / Match-V5 expose; never synthesize hidden timers or predictions presented as facts. A separate TUI window (not a game overlay) is also the safer posture.
2. **Required boilerplate** visible in-product: "[Product] is not endorsed by Riot Games…" (exact text in the policy). Only press-kit + game-specific static assets allowed as IP.
3. **Registration expectation**: products serving players must be registered even if they avoid official APIs — registering for a **personal key** satisfies this for a private-use tool; do not distribute publicly on a personal key.
4. **Non-public accounts**: private profiles make Summoner-V4 `by-name` and Match-V5 lookups fail (403/404) for *other* viewers; own-account flows and Spectator-V5 active-game detection are unaffected in practice. Handle 403/404 as "data unavailable", never as errors to retry.
5. **Rate-limit etiquette**: track `X-App-Rate-Limit(-Count)` per region; exponential backoff honoring `Retry-After` on 429; cache immutable things aggressively (finished matches/timelines never change — cache forever; static data per patch; summoner/ranked with short TTL); jitter + single-flight Spectator polls (≥30 s interval); expect occasional 503/long-hangs.
6. **Windows specifics**: both local APIs use Riot's self-signed cert — prefer embedding `riotgames.pem` as trusted root (pattern proven by `lol-game-client-api`) over blanket `danger_accept_invalid_certs`; port 2999 exists only mid-game (poll-with-backoff = lifecycle detector); loopback traffic needs no firewall elevation; process-cmdline credential discovery must use `Get-CimInstance` (wmic deprecated).
7. **Production-key trap**: if distribution ever goes public, plan a 6–9 month approval runway (Jan 2026 reports) — architecture must survive on caching + local-first data regardless.

## 7. Approach Comparison

| Approach | Description | Pros | Cons | Effort |
|---|---|---|---|---|
| **A. Live-first** (own game) | Port-2999 polling core; web API only for enrichment | True real-time; zero quota pressure; simplest auth story | Only works on user's machine mid-game; no enemy gold | Medium |
| **B. Post-game-first** | Match-V5/timeline analytics as core | Richest data (per-minute gold/positions, all 10); works anywhere | Not "live"; subject to upload delay/404s | Low-Medium |
| **C. Hybrid phased (recommended)** | MVP: A + B behind a shared metrics model; v2: LCU auto-detection + Spectator-V5 roster panel | Matches the product vision end-to-end; graceful degradation; each phase shippable | Slightly larger upfront abstraction (trait for sources) | High but incremental |

## 8. Affected Areas (repo)

- `E:\dev\TUI-LOL` (note: working dir `C:\Users\jseramn\Documents\pc\TUI-LOL` is an empty stub; the real repo is on E:).
- `Cargo.toml` / `src/` — fresh ratatui 0.30 + crossterm 0.29 bootstrap (commit `ed19205`); all integration code is greenfield.
- Candidate module seams (for design phase, not binding): `api/riot` (web API client + rate limiter), `api/live` (2999 client + cert), `sources` trait (live vs replay vs post-game providers), `domain/metrics` normalization, `ui/panels` (ratatui widgets), `store` (patch-keyed static cache + immutable match cache).
- Existing ecosystem leverage: `riven` (mature async Riot API wrapper w/ rate limiting) or hand-rolled `reqwest`+`serde`; `lol-game-client-api` models for port 2999.

## 9. Ready for Proposal

Yes. Recommended scope statement for proposal: *phased hybrid (Approach C)* — Phase 1 delivers live own-game panels (KDA/CS/level/items/respawn/objective event feed, 0.5–1 Hz) + match-ID post-game analytics (full 10-player, incl. per-minute gold from timeline); Phase 2 adds Spectator-V5 roster/ranked enrichment and LCU-driven auto-detection. Explicitly de-scope: live stats of third-party matches, any derived hidden-timer features.

## 10. Sources

- Portal docs (keys/rate limits/response codes/versioning): https://developer.riotgames.com/docs/portal
- LoL docs hub (routing, game client API, static data, developer policy): https://developer.riotgames.com/docs/lol
- APIs reference: https://developer.riotgames.com/apis
- General policies / terms: https://developer.riotgames.com/policies/general · https://developer.riotgames.com/terms
- FAQs: https://developer.riotgames.com/docs/faqs · https://support-developer.riotgames.com/hc/en-us/articles/22801164172051
- Routing explainer: https://darkintaqt.com/blog/routing · https://hextechdocs.dev/getting-started-with-the-riot-games-api/
- Live Client Data field models (Rust): https://docs.rs/lol-game-client-api/latest/src/lol_game_client_api/model.rs.html · api.rs (cert handling)
- Event-wrapper polling pattern: https://github.com/dysolix/ingame-api
- LCU guides: https://hextechdocs.dev/getting-started-with-the-lcu-api/ · https://hextechdocs.dev/getting-started-with-the-lcu-websocket/ · https://lcu.kebs.dev/
- Riven (Spectator-V5 surface, Rust): https://docs.rs/riven/latest/riven/endpoints/struct.SpectatorV5.html
- DevRel issue tracker: #477 (BR/LAN/LAS staleness) · #644/#807 (upload gaps/404s) · #923 (Arena) · #941 (KR) · #957 (retention/1000-cap) · #1064 (spectator grid unsupported) · #1126 (prod-key 6–9 mo backlog, Jan 2026)
- Data Dragon versions (fetched 2026-08-25): https://ddragon.leagueoflegends.com/api/versions.json
