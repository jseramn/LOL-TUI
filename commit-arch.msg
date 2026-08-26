chore(ops): track archived change artifacts and synced capability spec

Bookkeeping the archive phase was instructed to leave to the orchestrator:

  * `openspec/specs/live-dashboard-visualizations/spec.md` — the
    greenfield capability spec synced from the change folder to the
    canonical source-of-truth so future changes consume it directly.
  * `openspec/changes/add-live-visualizations/{proposal,specs,design,
    tasks,verify-report,exploration,archive-report}.md` moved into the
    dated archive `openspec/changes/archive/2026-08-26-add-live-
    visualizations/` by the archive phase (shell Move-Item; not a
    git-tracked rename so the diff shows as delete+add; the agent
    verified byte-identical destination contents before moving).

No code or test changes; pure ops commit mirroring the previous
change's archive bookkeeping.