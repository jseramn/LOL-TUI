//! crossterm input handling: `q`/Esc quit keys and terminal Resize events
//! (ui spec R1).
//!
//! Design D3 keeps the loop blocking and dependency-free: the shell polls
//! crossterm inline each frame via [`poll_actions`] and folds the returned
//! [`ShellAction`]s. All decision-making lives in the pure
//! [`classify_event`], which is what the tests exercise — no console needed.

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use std::io;
use std::time::Duration;

/// One shell-level action distilled from a raw terminal event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellAction {
    /// User pressed `q` or Esc — leave the app with exit status 0.
    Quit,
    /// Terminal was resized to the given bounds; the next frame redraws.
    Resize { width: u16, height: u16 },
}

/// Pure classifier: maps one raw event onto an optional [`ShellAction`].
///
/// Key handling accepts only `KeyEventKind::Press` — Windows terminals also
/// deliver Release events and acting on those would double-fire.
pub fn classify_event(event: &Event) -> Option<ShellAction> {
    match event {
        Event::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }) => match code {
            KeyCode::Char('q') | KeyCode::Esc => Some(ShellAction::Quit),
            _ => None,
        },
        Event::Resize(width, height) => {
            Some(ShellAction::Resize { width: *width, height: *height })
        }
        _ => None,
    }
}

/// Waits up to `timeout` for terminal input, then drains everything already
/// buffered. Returns every classified action in arrival order; unclassified
/// events (mouse, focus, other keys) are dropped.
///
/// # Errors
/// Propagates crossterm I/O failures from the underlying poll/read pair.
pub fn poll_actions(timeout: Duration) -> io::Result<Vec<ShellAction>> {
    let mut actions = Vec::new();
    if !event::poll(timeout)? {
        return Ok(actions);
    }
    loop {
        match event::read() {
            Ok(event) => {
                if let Some(action) = classify_event(&event) {
                    actions.push(action);
                }
            }
            Err(err) => return Err(err),
        }
        // Stop as soon as no more events are immediately pending so the
        // render loop keeps its per-frame cadence.
        if !event::poll(Duration::ZERO)? {
            return Ok(actions);
        }
    }
}
