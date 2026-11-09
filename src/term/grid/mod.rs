//! The screen: what the terminal shows right now, kept as a grid of cells.
//!
//! The parser (stage 2) turns the shell's bytes into [`Action`]s like "print
//! A" or "move the cursor to row 3". The grid *carries them out*. It keeps:
//!   - every cell on the screen (a letter plus its colours and styles);
//!   - the cursor: where the next letter goes, and whether it's shown;
//!   - the history ("scrollback"): lines that scrolled off the top;
//!   - a second, "alternate" screen that full-screen programs like `nano`
//!     and `htop` draw on, so your shell's screen is still there when they quit;
//!   - the window title, and a few switches ("modes") programs can flip.
//!
//! Nothing a program prints may crash Noob, so every position is clamped to the
//! screen and commands we don't know are ignored. Drawing the grid in the
//! window is stage 4; here we only keep track of what *would* be drawn.
//!
//! Rows and columns count from 0 in our code (row 0 is the top). Escape codes
//! count from 1 (`ESC[1;1H` is the top-left corner), so `csi.rs` converts.

mod cell;
mod csi;
mod resize;
mod sgr;
mod width;

use std::collections::VecDeque;

pub use cell::{Cell, Color, Look, Row, Width};

use super::parser::{Action, Control, Esc};

/// How many lines of history a new grid keeps. 10,000 lines of 200 columns is
/// about 50 MB at worst, and much less for normal output.
pub const DEFAULT_SCROLLBACK: usize = 10_000;

/// The biggest screen we accept, in rows or columns. Real windows are a few
/// hundred at most; a limit stops a bad size from using up all the memory.
const MAX_SIZE: usize = 1_000;

/// Tab stops are every 8 columns, like on a typewriter.
const TAB_WIDTH: usize = 8;

/// Switches programs can flip that the *keyboard* (stage 4) needs to know about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Modes {
    /// `ESC[?1h`: arrow keys send `ESC O A` instead of `ESC [ A` (vim and
    /// less ask for this).
    pub app_cursor_keys: bool,
    /// `ESC[?2004h`: pasted text is wrapped in markers, so the shell knows it was
    /// pasted and doesn't run each line as you paste it.
    pub bracketed_paste: bool,
}

/// Where the next letter goes, and the look it will have.
#[derive(Debug, Clone, Copy, Default)]
struct Cursor {
    row: usize,
    col: usize,
    /// Colours and styles for the next letters (set by `ESC[…m`).
    look: Look,
    /// The last letter filled the last column. The cursor stays on that column
    /// and the *next* letter wraps to a new line first. This "wait" means a full
    /// row followed by Enter doesn't leave an empty row in between.
    pending_wrap: bool,
}

/// What "save the cursor" (`ESC 7`) remembers.
#[derive(Debug, Clone, Copy)]
struct Saved {
    cursor: Cursor,
    origin: bool,
    autowrap: bool,
}

/// What the terminal shows. Feed it every [`Action`] from the parser, in order.
#[derive(Debug)]
pub struct Grid {
    rows: usize,
    cols: usize,
    /// The screen being shown: `rows` rows of `cols` cells.
    lines: Vec<Row>,
    /// The screen *not* being shown: the alternate one, or (while a program
    /// like nano runs) your shell's. Switching screens swaps the two.
    other: Vec<Row>,
    /// True while the alternate screen is the one shown.
    alternate: bool,
    /// Lines that scrolled off the top of the main screen; oldest first.
    scrollback: VecDeque<Row>,
    scrollback_limit: usize,
    cursor: Cursor,
    /// The saved cursor of the shown screen, and of the other one.
    saved: Option<Saved>,
    other_saved: Option<Saved>,
    cursor_visible: bool,
    /// The scroll region (`ESC[top;bottom r`): only these rows move when text
    /// scrolls. Normally the whole screen. Both rows are included.
    top: usize,
    bottom: usize,
    /// `tabs[col]` is true when there's a tab stop at that column.
    tabs: Vec<bool>,
    /// Wrap long lines onto the next row (on unless a program turns it off).
    autowrap: bool,
    /// "Origin mode": cursor positions count from the top of the scroll region.
    origin: bool,
    /// "Insert mode": new letters push the rest of the line right.
    insert: bool,
    modes: Modes,
    title: String,
    /// Answers for the shell (like "the cursor is at row 3, column 7"),
    /// waiting to be sent back to it.
    replies: Vec<u8>,
    /// The last letter printed, for "repeat the last letter" (`ESC[5b`).
    last_letter: Option<char>,
}

impl Grid {
    /// An empty screen of `rows` × `cols` cells (at least 1 × 1).
    pub fn new(rows: usize, cols: usize) -> Grid {
        let rows = rows.clamp(1, MAX_SIZE);
        let cols = cols.clamp(1, MAX_SIZE);
        Grid {
            rows,
            cols,
            lines: blank_screen(rows, cols),
            other: blank_screen(rows, cols),
            alternate: false,
            scrollback: VecDeque::new(),
            scrollback_limit: DEFAULT_SCROLLBACK,
            cursor: Cursor::default(),
            saved: None,
            other_saved: None,
            cursor_visible: true,
            top: 0,
            bottom: rows - 1,
            tabs: default_tabs(cols),
            autowrap: true,
            origin: false,
            insert: false,
            modes: Modes::default(),
            title: String::new(),
            replies: Vec::new(),
            last_letter: None,
        }
    }

    // ── Reading the screen (for drawing it, and for tests) ──────────────────

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    /// One cell of the shown screen, or `None` if that's off the screen.
    pub fn cell(&self, row: usize, col: usize) -> Option<&Cell> {
        self.lines.get(row)?.cells.get(col)
    }

    /// One row of the shown screen.
    pub fn row(&self, row: usize) -> Option<&Row> {
        self.lines.get(row)
    }

    /// Where the cursor is: `(row, column)`, counting from 0.
    pub fn cursor(&self) -> (usize, usize) {
        (self.cursor.row, self.cursor.col)
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// True while a full-screen program (nano, htop, vim) has the alternate screen.
    pub fn is_alternate(&self) -> bool {
        self.alternate
    }

    /// The window title the shell or program asked for (empty if none yet).
    pub fn title(&self) -> &str {
        &self.title
    }

    pub fn modes(&self) -> Modes {
        self.modes
    }

    /// How many lines of history there are.
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// One line of history; 0 is the oldest. Lines keep the width they had
    /// when they scrolled away, so they can be shorter or longer than `cols`.
    pub fn scrollback_row(&self, index: usize) -> Option<&Row> {
        self.scrollback.get(index)
    }

    /// Changes how many lines of history are kept (0 keeps none).
    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.scrollback_limit = limit;
        while self.scrollback.len() > limit {
            self.scrollback.pop_front();
        }
    }

    /// The shown screen as plain text: one string per row, spaces at the end
    /// dropped. Handy for tests and, later, for copying text.
    pub fn screen_text(&self) -> Vec<String> {
        self.lines.iter().map(Row::text).collect()
    }

    /// The history as plain text, oldest line first.
    pub fn scrollback_text(&self) -> Vec<String> {
        self.scrollback.iter().map(Row::text).collect()
    }

    /// Takes the answers waiting for the shell. Whoever owns the pseudo-terminal
    /// should write these bytes to it (ConPTY on Windows waits for one).
    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.replies)
    }

    // ── Carrying out actions ────────────────────────────────────────────────

    /// Does what one parser action asks.
    pub fn apply(&mut self, action: &Action) {
        match action {
            Action::Print(letter) => self.print(*letter),
            Action::Control(control) => self.control(*control),
            Action::Csi(csi) => self.csi(csi),
            Action::Esc(esc) => self.esc(esc),
            Action::Osc(parts) => self.osc(parts),
        }
    }

    /// Puts a letter at the cursor and moves the cursor past it.
    fn print(&mut self, letter: char) {
        let width = width::letter_width(letter);
        // Combining marks and invisible joiners take no room. We don't draw
        // them yet (stage 4 may join them to the letter before).
        // A wide letter can't be shown at all in a 1-column window.
        if width == 0 || width > self.cols {
            return;
        }
        if self.cursor.pending_wrap {
            self.wrap_line();
        }
        // A wide letter that doesn't fit on this row goes to the next one whole.
        if width == 2 && self.cursor.col + 1 >= self.cols {
            if self.autowrap {
                self.wrap_line();
            } else {
                self.cursor.col = self.cols - 2;
            }
        }
        if self.insert {
            self.insert_blanks(width);
        }

        let (row, col) = (self.cursor.row, self.cursor.col);
        let look = self.cursor.look;
        let line = &mut self.lines[row];
        if width == 2 {
            line.cells[col] = Cell {
                letter,
                look,
                width: Width::Two,
            };
            line.cells[col + 1] = Cell {
                letter: ' ',
                look,
                width: Width::Covered,
            };
        } else {
            line.cells[col] = Cell {
                letter,
                look,
                width: Width::One,
            };
        }
        // We may have written over half of an older wide letter: tidy it up.
        line.repair_wide(col.saturating_sub(1), col + width + 1);
        self.last_letter = Some(letter);

        let next = col + width;
        if next >= self.cols {
            self.cursor.col = self.cols - 1;
            self.cursor.pending_wrap = self.autowrap;
        } else {
            self.cursor.col = next;
        }
    }

    /// The line is full: mark it as carrying on, and go to the next one.
    fn wrap_line(&mut self) {
        self.lines[self.cursor.row].wrapped = true;
        self.cursor.col = 0;
        self.line_feed();
    }

    fn control(&mut self, control: Control) {
        match control {
            Control::Backspace => {
                self.cursor.pending_wrap = false;
                self.cursor.col = self.cursor.col.saturating_sub(1);
            }
            Control::Tab => self.tab_forward(1),
            Control::LineFeed => self.line_feed(),
            Control::CarriageReturn => {
                self.cursor.pending_wrap = false;
                self.cursor.col = 0;
            }
            // The bell will flash the window in stage 4; other codes mean nothing.
            Control::Bell | Control::Other(_) => {}
        }
    }

    fn esc(&mut self, esc: &Esc) {
        match (esc.intermediates.as_str(), esc.final_char) {
            ("", '7') => self.save_cursor(),
            ("", '8') => self.restore_cursor(),
            // "Index": down one line, scrolling if at the bottom.
            ("", 'D') => self.line_feed(),
            // "Next line": like Enter.
            ("", 'E') => {
                self.cursor.col = 0;
                self.line_feed();
            }
            // "Reverse index": up one line, scrolling down if at the top.
            ("", 'M') => self.reverse_line_feed(),
            // Set a tab stop where the cursor is.
            ("", 'H') => self.tabs[self.cursor.col] = true,
            ("", 'c') => self.reset(),
            // Character sets (`ESC ( B`), keypad modes…: nothing to do.
            _ => {}
        }
    }

    /// An "Operating System Command". `0` and `2` set the window title.
    fn osc(&mut self, parts: &[String]) {
        if let Some((kind, rest)) = parts.split_first()
            && (kind == "0" || kind == "2")
        {
            // The title itself may contain `;`, so glue the pieces back together.
            self.title = rest.join(";");
        }
    }

    /// `ESC c`: back to how a new terminal starts. The history is kept.
    fn reset(&mut self) {
        let scrollback = std::mem::take(&mut self.scrollback);
        let replies = std::mem::take(&mut self.replies);
        let limit = self.scrollback_limit;
        *self = Grid {
            scrollback,
            replies,
            scrollback_limit: limit,
            ..Grid::new(self.rows, self.cols)
        };
    }

    // ── Moving down and up, and scrolling ───────────────────────────────────

    /// Down one row. At the bottom of the scroll region, everything in the
    /// region moves up instead and an empty row appears at the bottom.
    fn line_feed(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.bottom {
            self.scroll_up(1);
        } else if self.cursor.row + 1 < self.rows {
            self.cursor.row += 1;
        }
    }

    /// Up one row; at the top of the scroll region, scroll down instead.
    fn reverse_line_feed(&mut self) {
        self.cursor.pending_wrap = false;
        if self.cursor.row == self.top {
            self.scroll_down(1);
        } else {
            self.cursor.row = self.cursor.row.saturating_sub(1);
        }
    }

    /// Moves the scroll region's rows up by `count`. Rows leaving the top of
    /// the main screen go into the history.
    fn scroll_up(&mut self, count: usize) {
        let count = count.min(self.bottom - self.top + 1);
        let to_history = self.top == 0 && !self.alternate;
        for _ in 0..count {
            let gone = self.lines.remove(self.top);
            self.lines.insert(self.bottom, self.blank_row());
            if to_history {
                self.keep_in_history(gone);
            }
        }
    }

    /// Moves the scroll region's rows down by `count`; the bottom ones vanish.
    fn scroll_down(&mut self, count: usize) {
        let count = count.min(self.bottom - self.top + 1);
        for _ in 0..count {
            self.lines.remove(self.bottom);
            self.lines.insert(self.top, self.blank_row());
        }
    }

    fn keep_in_history(&mut self, row: Row) {
        if self.scrollback_limit == 0 {
            return;
        }
        if self.scrollback.len() >= self.scrollback_limit {
            self.scrollback.pop_front();
        }
        self.scrollback.push_back(row);
    }

    // ── Small helpers ───────────────────────────────────────────────────────

    /// The look of erased cells: the current background colour, nothing else.
    fn blank_look(&self) -> Look {
        Look {
            bg: self.cursor.look.bg,
            ..Look::default()
        }
    }

    fn blank_row(&self) -> Row {
        Row::blank(self.cols, self.blank_look())
    }

    /// Pushes the rest of the cursor's row right by `count` empty cells.
    fn insert_blanks(&mut self, count: usize) {
        let (row, col) = (self.cursor.row, self.cursor.col);
        let count = count.min(self.cols - col);
        let blank = Cell::blank(self.blank_look());
        let cells = &mut self.lines[row].cells;
        cells.truncate(self.cols - count);
        cells.splice(col..col, std::iter::repeat_n(blank, count));
        self.lines[row].repair_wide(0, self.cols);
    }

    fn tab_forward(&mut self, count: usize) {
        self.cursor.pending_wrap = false;
        for _ in 0..count.min(self.cols) {
            let next = (self.cursor.col + 1..self.cols).find(|col| self.tabs[*col]);
            self.cursor.col = next.unwrap_or(self.cols - 1);
        }
    }

    fn tab_back(&mut self, count: usize) {
        self.cursor.pending_wrap = false;
        for _ in 0..count.min(self.cols) {
            let previous = (0..self.cursor.col).rev().find(|col| self.tabs[*col]);
            self.cursor.col = previous.unwrap_or(0);
        }
    }

    fn save_cursor(&mut self) {
        self.saved = Some(Saved {
            cursor: self.cursor,
            origin: self.origin,
            autowrap: self.autowrap,
        });
    }

    /// Brings back the saved cursor (or goes home, if nothing was saved).
    fn restore_cursor(&mut self) {
        let saved = self.saved.unwrap_or(Saved {
            cursor: Cursor::default(),
            origin: false,
            autowrap: true,
        });
        self.cursor = saved.cursor;
        self.cursor.row = self.cursor.row.min(self.rows - 1);
        self.cursor.col = self.cursor.col.min(self.cols - 1);
        self.origin = saved.origin;
        self.autowrap = saved.autowrap;
    }

    /// Shows the alternate screen (`on`) or the main one. With `clear`, the
    /// alternate screen starts empty.
    fn switch_screen(&mut self, on: bool, clear: bool) {
        if on == self.alternate {
            return;
        }
        std::mem::swap(&mut self.lines, &mut self.other);
        std::mem::swap(&mut self.saved, &mut self.other_saved);
        self.alternate = on;
        self.cursor.pending_wrap = false;
        if on && clear {
            self.lines = blank_screen(self.rows, self.cols);
        }
    }
}

/// `rows` empty rows of `cols` cells.
fn blank_screen(rows: usize, cols: usize) -> Vec<Row> {
    vec![Row::blank(cols, Look::default()); rows]
}

/// A tab stop every `TAB_WIDTH` columns (not at column 0).
fn default_tabs(cols: usize) -> Vec<bool> {
    (0..cols)
        .map(|col| col > 0 && col % TAB_WIDTH == 0)
        .collect()
}

#[cfg(test)]
mod tests;
