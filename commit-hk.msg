chore(ops): reconcile working tree after archive move and apply progress

Working-tree reconciliation needed before U6 (fmt/clippy) can run clean:

  * `openspec/changes/add-live-match-dashboard/*` deleted from working
    tree by the sdd-archive phase (the agent moved them into
    `openspec/changes/archive/2026-08-25-add-live-match-dashboard/`
    and synced specs to `openspec/specs/`); stage the deletions plus the
    new archive/specs tree in one commit so git history shows the move.
  * `openspec/changes/add-live-visualizations/tasks.md` picked up
    checkbox marks from U1–U5 apply passes.
  * `tests/ui/glyph_tests.rs` had a two-line fmt drift picked up by
    rustfmt; normalize before fmt --check runs on U6.

No code or test logic changes. Pure ops commit.