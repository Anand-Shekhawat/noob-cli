//! One running shell: the pseudo-terminal, our parser and our screen grid,
//! joined together.
//!
//!   shell ──bytes──▶ parser ──actions──▶ grid ──▶ drawn in the window
//!     ▲                                    │
//!     └──── keys you type, and the grid's answers ("cursor is at 3;7")
//!
//! A helper thread (inside `Pty`) waits for the shell's output and wakes the
//! window when some arrives. The window then calls `pump`, which feeds it all
//! through the parser into the grid.

use std::path::PathBuf;
use std::time::Duration;

use eframe::egui;
use noob_cli::term::folder;
use noob_cli::term::grid::Grid;
use noob_cli::term::parser::Parser;
use noob_cli::term::pty::{Pty, Size};

pub struct Session {
    pty: Pty,
    parser: Parser,
    grid: Grid,
    /// Set once the shell has ended: a sentence saying so.
    ended: Option<String>,
}

impl Session {
    /// Starts your shell at `rows` × `cols`. `ctx` is woken whenever it prints.
    pub fn start(rows: usize, cols: usize, ctx: &egui::Context) -> Result<Session, String> {
        let ctx = ctx.clone();
        let pty = Pty::start_shell_with_wake(
            pty_size(rows, cols),
            Box::new(move || ctx.request_repaint()),
        )?;
        Ok(Session {
            pty,
            parser: Parser::new(),
            grid: Grid::new(rows, cols),
            ended: None,
        })
    }

    pub fn grid(&self) -> &Grid {
        &self.grid
    }

    /// Why the shell is no longer running, if it isn't.
    pub fn ended(&self) -> Option<&str> {
        self.ended.as_deref()
    }

    /// Takes in everything the shell printed since last time. True if the
    /// screen changed (so the blinking cursor should show straight away).
    pub fn pump(&mut self) -> bool {
        let changed = self.take_output();
        if self.ended.is_none()
            && let Some(status) = self.pty.wait_for_exit(Duration::ZERO)
        {
            // Anything printed just before the end still belongs on screen.
            self.take_output();
            self.ended = Some(if status.success() {
                "The shell has ended.".to_string()
            } else {
                format!("The shell has ended (exit code {}).", status.exit_code())
            });
            return true;
        }
        changed
    }

    fn take_output(&mut self) -> bool {
        let Some(bytes) = self.pty.read_timeout(Duration::ZERO) else {
            return false;
        };
        for action in self.parser.feed(&bytes) {
            self.grid.apply(&action);
        }
        // Questions like "where is your cursor?" must be answered, or some
        // programs (and Windows' ConPTY) wait forever.
        let replies = self.grid.take_replies();
        if !replies.is_empty() {
            self.write(&replies);
        }
        true
    }

    /// The folder the shell is in right now (unknown on Windows).
    pub fn folder(&self) -> Option<PathBuf> {
        self.pty.process_id().and_then(folder::of_process)
    }

    /// Why the shell can't take a typed command right now, if it can't.
    pub fn busy(&self) -> Option<&'static str> {
        if self.ended.is_some() {
            return Some("The shell has ended. Start a new one first.");
        }
        if self.grid.is_alternate() {
            return Some("A program is open in the terminal. Close it first (in nano: Ctrl+X).");
        }
        let shell = self.pty.process_id();
        let in_front = self.pty.foreground_process();
        if shell.is_some() && in_front.is_some() && shell != in_front {
            return Some(
                "The terminal is still running something. Wait for it, or press Ctrl+C there.",
            );
        }
        None
    }

    /// Sends bytes to the shell as if typed. Does nothing once it has ended.
    pub fn write(&mut self, bytes: &[u8]) {
        if self.ended.is_some() {
            return;
        }
        // A failed write means the shell is going away; `pump` will notice.
        let _ = self.pty.write(bytes);
    }

    /// Makes the screen `rows` × `cols`, and tells the shell so it can redraw.
    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.grid.resize(rows, cols);
        // If the shell is gone there's nobody to tell; the grid still fits the window.
        let _ = self.pty.resize(pty_size(rows, cols));
    }
}

fn pty_size(rows: usize, cols: usize) -> Size {
    let clamp = |count: usize| u16::try_from(count.max(1)).unwrap_or(u16::MAX);
    Size {
        rows: clamp(rows),
        cols: clamp(cols),
    }
}
