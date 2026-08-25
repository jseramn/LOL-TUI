//! Binary entry point: poller thread, panic-safe terminal lifecycle, and the
//! blocking render loop (design D3 + "Terminal Handling").
//!
//! Guarantees:
//! - Raw mode + alternate screen are entered before the first frame.
//! - Restoration is guaranteed by BOTH a [`Drop`]-based [`TerminalGuard`]
//!   (normal exits and `?`-propagated errors) and a panic hook that restores
//!   the console first, then delegates to the previous hook — returning from
//!   the hook resumes unwinding, so panics can never orphan a broken console.
//! - `q` / Esc exits cleanly with status 0 (ui spec R1/S1).
//! - Resizes need no special casing here: every frame goes through
//!   `Terminal::draw`, which autoresizes to the backend's current bounds
//!   (proven offline by `tests/ui/shell_render_tests.rs`).

use std::io::{self, Write};
use std::panic;
use std::process::ExitCode;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tui_lol::api::client::{ApiClient, LIVE_CLIENT_PORT, LOOPBACK_HOST};
use tui_lol::api::poller::{DEFAULT_CADENCE, PollMsg, Poller, SystemClock};
use tui_lol::app::App;
use tui_lol::events::{ShellAction, poll_actions};
use tui_lol::ui;

/// Per-frame input-poll window: bounds quit latency and doubles as the UI
/// tick so standby/live frames refresh even without new data.
const FRAME_POLL: Duration = Duration::from_millis(50);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("tui-lol: {err}");
            ExitCode::FAILURE
        }
    }
}

/// RAII console state: dropping it always leaves raw mode and the alternate
/// screen, whether the loop returned normally or via `?`.
struct TerminalGuard;

impl TerminalGuard {
    /// # Errors
    /// Propagates crossterm failures while entering raw mode or switching
    /// screens. Nothing was switched on if either fails.
    fn enter() -> io::Result<Self> {
        let mut stdout = io::stdout();
        // Flush any pending prints BEFORE capturing the screen so startup
        // output stays on the primary buffer.
        stdout.flush()?;
        enable_raw_mode()?;
        if let Err(err) = execute!(stdout, EnterAlternateScreen) {
            let _ = disable_raw_mode(); // don't leave half-entered state behind
            return Err(err);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

/// Installs a panic hook that restores the console FIRST, then runs the
/// previous hook for the standard report. Returning from a hook resumes
/// unwinding, satisfying the design requirement "restore, then resume".
fn install_panic_restore_hook() {
    let previous_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        previous_hook(info);
    }));
}

/// Spawns the single poller thread (design D3 — no async runtime) sampling
/// `/liveclientdata/allgamedata` at the clamped cadence. Returns the receive
/// side; the thread parks itself the moment the shell drops `rx`.
fn spawn_poller_thread() -> mpsc::Receiver<PollMsg> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("live-client-poller".into())
        .spawn(move || {
            // Cannot fail: the host is the pinned loopback literal guarded at
            // construction (poller spec R1/S3).
            let client =
                ApiClient::new(LOOPBACK_HOST, LIVE_CLIENT_PORT).expect("loopback host is valid");
            let mut poller = Poller::new(client, SystemClock, DEFAULT_CADENCE);
            loop {
                for msg in poller.run_once() {
                    if tx.send(msg).is_err() {
                        // Shell closed — stop polling, end the thread.
                        return;
                    }
                }
            }
        })
        .expect("thread spawn with default settings cannot fail");
    rx
}

/// # Errors
/// Propagates terminal I/O failures from setup, event polling, or drawing.
fn run() -> io::Result<()> {
    install_panic_restore_hook();

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let rx = spawn_poller_thread();
    let mut app = App::new();

    loop {
        // Input first so quit feels instant even under data pressure.
        for action in poll_actions(FRAME_POLL)? {
            match action {
                ShellAction::Quit => return Ok(()),
                ShellAction::Resize { .. } => {} // No explicit work: the draw below autoresizes next frame.
            }
        }

        // Latest-wins fold of everything the poller produced since last frame.
        app.drain(&rx);

        terminal.draw(|frame| ui::render(frame, &app))?;
    }
}
