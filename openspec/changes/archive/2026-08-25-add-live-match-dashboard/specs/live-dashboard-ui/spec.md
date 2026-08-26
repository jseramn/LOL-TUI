# Live Dashboard UI Specification

## Purpose

Terminal user interface (ratatui/crossterm) that renders live match state produced by the live-client-poller: a standby view when no game is detected, and during the user's game a dashboard with panels for all 10 players, an enhanced local-player panel, an objective & kill event ticker, and a status line carrying the mandatory Riot Games non-endorsement notice. Rendering SHALL degrade gracefully wherever the API does not expose a field.

## Requirements

### Requirement: Terminal shell and event loop

The application SHALL run a crossterm-driven render loop under both Windows Terminal and classic conhost on Windows (non-Windows console hosts are out of scope). It SHALL redraw on data updates, tick, and terminal resize. Pressing `q` or `Esc` SHALL exit and restore the terminal to its pre-launch state.

#### Scenario: Clean startup and quit

- GIVEN the application launched in raw mode showing the standby view
- WHEN the user presses `q`
- THEN the process exits with success status and the terminal is fully restored (raw mode disabled, alternate screen left)

#### Scenario: Terminal resize reflows without failure

- GIVEN the live dashboard is displayed
- WHEN a resize event shrinks the terminal
- THEN the next frame is drawn within the new bounds, no element panics, and the Riot notice remains visible

### Requirement: Idle / standby view

When the poller reports NOT_IN_GAME, the UI SHALL display a standby view indicating it is waiting for a live game. It MUST NOT enter an error loop or spam error output while no game is running.

#### Scenario: Startup outside a game

- GIVEN no game is running when the app starts
- WHEN the first frames render
- THEN the standby view is shown with no unhandled-error output

#### Scenario: Mid-game disconnect falls back to standby

- GIVEN the live dashboard is active
- WHEN the poller emits IN_GAME → NOT_IN_GAME
- THEN the UI switches to the standby view without crashing

#### Scenario: Reconnect returns to the live view

- GIVEN the standby view after a disconnect
- WHEN the poller emits NOT_IN_GAME → IN_GAME with a fresh snapshot
- THEN the live dashboard is restored within one poll cycle (≤ 1 s at default cadence)

### Requirement: All-player live panels

During IN_GAME, the UI SHALL render one panel per player — all 10 — grouped by team, showing champion name, level, KDA, creep score, items, summoner spells (with cooldowns as exposed), death state, and respawn timer as exposed. The latest received snapshot SHALL be reflected in the next rendered frame.

#### Scenario: Complete snapshot renders all players

- GIVEN a snapshot with complete data for all 10 players
- WHEN a frame renders
- THEN each of the 10 panels shows every listed field with the snapshot's values

#### Scenario: Partial player fields degrade per field

- GIVEN a snapshot where one player lacks items and respawnTimer
- WHEN a frame renders
- THEN that panel shows explicit placeholders for the absent fields and all other panels remain fully populated

#### Scenario: Dead player shows exposed respawn value only

- GIVEN a player with isDead true and respawnTimer 21.3
- WHEN a frame renders
- THEN the panel shows the exposed respawn value; if isDead is true but respawnTimer is absent, the panel shows an unknown marker instead of any derived countdown

### Requirement: Local-player enhanced panel

The UI SHALL distinguish the local player's panel and additionally display currentGold and other locally exposed stat detail. It MUST NOT render any gold value for non-local players.

#### Scenario: Local gold rendered when exposed

- GIVEN a snapshot whose local player carries currentGold 4350
- WHEN a frame renders
- THEN the local panel shows 4350 gold

#### Scenario: Missing local gold degrades; enemy gold never shown

- GIVEN a snapshot with currentGold absent for the local player and no gold for others
- WHEN a frame renders
- THEN the local panel shows a placeholder and no panel but the local one displays any gold value

### Requirement: Objective and kill event ticker

The UI SHALL maintain a ticker of match events from the snapshot event list — ChampionKill, FirstBlood, Multikill, Ace, TurretKilled, InhibKilled/InhibRespawned, DragonKill (with dragon type and stolen flag), HeraldKill, BaronKill, GameStart, GameEnd — showing type, participants, and event time as exposed.

#### Scenario: Events appear in the ticker

- GIVEN a snapshot whose events include FirstBlood, TurretKilled, and DragonKill (Chemtech, stolen)
- WHEN a frame renders
- THEN the ticker lists all three with their participants and times as recorded

#### Scenario: Empty event list shows placeholder

- GIVEN a snapshot with zero events
- WHEN a frame renders
- THEN the ticker area shows an empty-state message instead of failing

### Requirement: Mandatory non-endorsement notice and status line

Every view — idle or live — SHALL persistently display a notice stating that this product is not endorsed by Riot Games, together with a status line showing connection/lifecycle state and last update time.

#### Scenario: Notice visible in every state

- GIVEN the app in standby view, then again in live view
- WHEN frames render in each state
- THEN the notice containing "not endorsed by Riot Games" and the lifecycle status are visible in both
