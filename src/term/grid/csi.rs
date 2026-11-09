//! Carrying out Control Sequences (`ESC [ … letter`): cursor moves, erasing,
//! inserting and deleting, scrolling, colours and modes.
//!
//! The common ones, with their traditional short names (you'll see these in
//! every terminal's documentation):
//!
//! | Letter  | Name            | What it does                                   |
//! |---------|-----------------|------------------------------------------------|
//! | A B C D | CUU CUD CUF CUB | cursor up, down, right, left `n` steps         |
//! | E F     | CNL CPL         | down / up `n` lines, to the start of the line  |
//! | G `     | CHA HPA         | go to column `n`                               |
//! | d       | VPA             | go to row `n`                                  |
//! | H f     | CUP             | go to row, column                              |
//! | J       | ED              | erase the display (below, above, all, history) |
//! | K       | EL              | erase the line (right, left, all)              |
//! | @ P X   | ICH DCH ECH     | insert, delete, erase `n` letters              |
//! | L M     | IL DL           | insert, delete `n` lines                       |
//! | S T     | SU SD           | scroll up, down `n` lines                      |
//! | r       | DECSTBM         | set the scroll region (top; bottom)            |
//! | s u     |                 | save, restore the cursor                       |
//! | m       | SGR             | colours and styles (see `sgr.rs`)              |
//! | h l     | SM RM           | switch modes on / off (`?` ones are DEC's)     |
//!
//! Numbers count from 1, and a missing number (or 0) means "the usual": 1 for
//! counts and positions.

use super::{Cell, Grid, MAX_REPLY_BYTES, sgr};
use crate::term::parser::Csi;

impl Grid {
    pub(super) fn csi(&mut self, csi: &Csi) {
        let numbers = csi.numbers();
        // The `index`th number, where 0 or missing means 1 (for counts and positions).
        let count = |index: usize| -> usize {
            numbers
                .get(index)
                .copied()
                .filter(|number| *number != 0)
                .map_or(1, usize::from)
        };
        // The `index`th number as it was sent (missing means 0).
        let raw = |index: usize| -> u16 { numbers.get(index).copied().unwrap_or(0) };

        match (csi.private, csi.intermediates.as_str(), csi.final_char) {
            (None, "", 'A') => self.cursor_up(count(0)),
            (None, "", 'B' | 'e') => self.cursor_down(count(0)),
            (None, "", 'C' | 'a') => self.cursor_right(count(0)),
            (None, "", 'D') => self.cursor_left(count(0)),
            (None, "", 'E') => {
                self.cursor_down(count(0));
                self.cursor.col = 0;
            }
            (None, "", 'F') => {
                self.cursor_up(count(0));
                self.cursor.col = 0;
            }
            (None, "", 'G' | '`') => self.go_to(self.cursor.row, count(0) - 1),
            (None, "", 'd') => self.go_to_in_region(count(0) - 1, self.cursor.col),
            (None, "", 'H' | 'f') => self.go_to_in_region(count(0) - 1, count(1) - 1),
            (None, "", 'I') => self.tab_forward(count(0)),
            (None, "", 'Z') => self.tab_back(count(0)),
            (None, "", 'J') => self.erase_display(raw(0)),
            (None, "", 'K') => self.erase_line(raw(0)),
            (None, "", '@') => {
                self.cursor.pending_wrap = false;
                self.insert_blanks(count(0));
            }
            (None, "", 'P') => self.delete_letters(count(0)),
            (None, "", 'X') => self.erase_letters(count(0)),
            (None, "", 'L') => self.insert_lines(count(0)),
            (None, "", 'M') => self.delete_lines(count(0)),
            (None, "", 'S') => self.scroll_up(count(0)),
            (None, "", 'T') => self.scroll_down(count(0)),
            (None, "", 'b') => self.repeat_letter(count(0)),
            (None, "", 'g') => self.clear_tabs(raw(0)),
            (None, "", 'm') => sgr::apply(&mut self.cursor.look, &csi.params),
            (None, "", 'r') => self.set_scroll_region(count(0), raw(1)),
            (None, "", 's') if numbers.is_empty() => self.save_cursor(),
            (None, "", 'u') => self.restore_cursor(),
            (None, "", 'h') => self.set_modes(&numbers, true),
            (None, "", 'l') => self.set_modes(&numbers, false),
            (Some('?'), "", 'h') => self.set_dec_modes(&numbers, true),
            (Some('?'), "", 'l') => self.set_dec_modes(&numbers, false),
            (None, "", 'n') => self.report(raw(0)),
            // "What kind of terminal are you?" We say: a VT100-style one.
            (None, "", 'c') if raw(0) == 0 => self.reply("\x1b[?1;2c"),
            (Some('>'), "", 'c') if raw(0) == 0 => self.reply("\x1b[>0;10;1c"),
            // Cursor shapes, window tricks, mouse reports…: not yet, so ignored.
            _ => {}
        }
    }

    // ── Cursor moves ────────────────────────────────────────────────────────

    /// Up `count` rows. Inside the scroll region it stops at the region's top.
    fn cursor_up(&mut self, count: usize) {
        let stop = if self.cursor.row >= self.top {
            self.top
        } else {
            0
        };
        let row = self.cursor.row.saturating_sub(count).max(stop);
        self.go_to(row, self.cursor.col);
    }

    /// Down `count` rows. Inside the scroll region it stops at the region's bottom.
    fn cursor_down(&mut self, count: usize) {
        let stop = if self.cursor.row <= self.bottom {
            self.bottom
        } else {
            self.rows - 1
        };
        let row = self.cursor.row.saturating_add(count).min(stop);
        self.go_to(row, self.cursor.col);
    }

    fn cursor_right(&mut self, count: usize) {
        self.go_to(self.cursor.row, self.cursor.col.saturating_add(count));
    }

    fn cursor_left(&mut self, count: usize) {
        self.go_to(self.cursor.row, self.cursor.col.saturating_sub(count));
    }

    /// Moves the cursor to a screen position, kept on the screen.
    fn go_to(&mut self, row: usize, col: usize) {
        self.cursor.row = row.min(self.rows - 1);
        self.cursor.col = col.min(self.cols - 1);
        self.cursor.pending_wrap = false;
    }

    /// Like `go_to`, but in origin mode the row counts from the top of the
    /// scroll region and can't leave it.
    fn go_to_in_region(&mut self, row: usize, col: usize) {
        if self.origin {
            let row = self.top.saturating_add(row).min(self.bottom);
            self.go_to(row, col);
        } else {
            self.go_to(row, col);
        }
    }

    // ── Erasing ─────────────────────────────────────────────────────────────

    /// ED: 0 = from the cursor to the end, 1 = from the start to the cursor,
    /// 2 = the whole screen, 3 = the history.
    fn erase_display(&mut self, mode: u16) {
        let row = self.cursor.row;
        match mode {
            0 => {
                self.erase_line(0);
                for below in row + 1..self.rows {
                    self.lines[below] = self.blank_row();
                }
            }
            1 => {
                for above in 0..row {
                    self.lines[above] = self.blank_row();
                }
                self.erase_line(1);
            }
            2 => {
                for any in 0..self.rows {
                    self.lines[any] = self.blank_row();
                }
            }
            3 => self.scrollback.clear(),
            _ => {}
        }
    }

    /// EL: 0 = from the cursor to the end, 1 = from the start to the cursor,
    /// 2 = the whole line.
    fn erase_line(&mut self, mode: u16) {
        let col = self.cursor.col;
        let (from, to) = match mode {
            0 => (col, self.cols),
            1 => (0, col + 1),
            2 => (0, self.cols),
            _ => return,
        };
        self.blank_cells(from, to);
        if mode != 1 {
            // The end of the line is gone, so it no longer carries on.
            self.lines[self.cursor.row].wrapped = false;
        }
    }

    /// ECH: blanks `count` letters from the cursor, without moving anything.
    fn erase_letters(&mut self, count: usize) {
        let col = self.cursor.col;
        self.blank_cells(col, col.saturating_add(count));
    }

    /// Blanks the cursor row's cells from `from` up to (not including) `to`.
    fn blank_cells(&mut self, from: usize, to: usize) {
        self.cursor.pending_wrap = false;
        let to = to.min(self.cols);
        let blank = Cell::blank(self.blank_look());
        let line = &mut self.lines[self.cursor.row];
        line.cells[from..to].fill(blank);
        line.repair_wide(0, self.cols);
    }

    // ── Inserting and deleting ──────────────────────────────────────────────

    /// DCH: removes `count` letters at the cursor; the rest of the line moves left.
    fn delete_letters(&mut self, count: usize) {
        self.cursor.pending_wrap = false;
        let col = self.cursor.col;
        let count = count.min(self.cols - col);
        let blank = Cell::blank(self.blank_look());
        let line = &mut self.lines[self.cursor.row];
        line.cells.drain(col..col + count);
        line.cells.extend(std::iter::repeat_n(blank, count));
        line.repair_wide(0, self.cols);
    }

    /// IL: pushes the rows from the cursor down by `count` empty rows (only
    /// inside the scroll region; rows pushed past its bottom vanish).
    fn insert_lines(&mut self, count: usize) {
        let row = self.cursor.row;
        if row < self.top || row > self.bottom {
            return;
        }
        for _ in 0..count.min(self.bottom - row + 1) {
            self.lines.remove(self.bottom);
            self.lines.insert(row, self.blank_row());
        }
        self.go_to(row, 0);
    }

    /// DL: removes `count` rows at the cursor; the rows below move up.
    fn delete_lines(&mut self, count: usize) {
        let row = self.cursor.row;
        if row < self.top || row > self.bottom {
            return;
        }
        for _ in 0..count.min(self.bottom - row + 1) {
            self.lines.remove(row);
            self.lines.insert(self.bottom, self.blank_row());
        }
        self.go_to(row, 0);
    }

    /// REP: prints the last letter `count` more times.
    fn repeat_letter(&mut self, count: usize) {
        if let Some(letter) = self.last_letter {
            // More than a screenful of the same letter looks the same as a screenful.
            for _ in 0..count.min(self.rows * self.cols) {
                self.print(letter);
            }
        }
    }

    // ── Tabs, regions, modes, answers ───────────────────────────────────────

    /// TBC: 0 = remove the tab stop at the cursor, 3 = remove them all.
    fn clear_tabs(&mut self, mode: u16) {
        match mode {
            0 => self.tabs[self.cursor.col] = false,
            3 => self.tabs.fill(false),
            _ => {}
        }
    }

    /// DECSTBM: only rows `top` to `bottom` (counting from 1) scroll from now
    /// on. A missing bottom means the last row. Then the cursor goes home.
    fn set_scroll_region(&mut self, top: usize, bottom: u16) {
        let top = top - 1;
        let bottom = match bottom {
            0 => self.rows - 1,
            row => (usize::from(row) - 1).min(self.rows - 1),
        };
        // A region needs at least two rows, top above bottom.
        if top >= bottom {
            return;
        }
        self.top = top;
        self.bottom = bottom;
        self.go_to_in_region(0, 0);
    }

    /// SM / RM: the few standard modes we know.
    fn set_modes(&mut self, numbers: &[u16], on: bool) {
        for number in numbers {
            if *number == 4 {
                self.insert = on;
            }
        }
    }

    /// DEC private modes (`ESC[?…h` / `ESC[?…l`).
    fn set_dec_modes(&mut self, numbers: &[u16], on: bool) {
        for number in numbers {
            match number {
                1 => self.modes.app_cursor_keys = on,
                6 => {
                    self.origin = on;
                    self.go_to_in_region(0, 0);
                }
                7 => {
                    self.autowrap = on;
                    self.cursor.pending_wrap = false;
                }
                25 => self.cursor_visible = on,
                // The alternate screen, the old way (nothing saved).
                47 => self.switch_screen(on, false),
                1047 => self.switch_screen(on, true),
                1048 if on => self.save_cursor(),
                1048 => self.restore_cursor(),
                // The alternate screen, the way nano, htop and vim use it: save
                // the cursor, switch, start empty; on the way out, switch back
                // and bring the cursor back.
                1049 if on => {
                    self.save_cursor();
                    self.switch_screen(true, true);
                }
                // Only bring the cursor back if we really were on the alternate
                // screen; a stray "leave" must not move it.
                1049 if self.alternate => {
                    self.switch_screen(false, false);
                    self.restore_cursor();
                }
                2004 => self.modes.bracketed_paste = on,
                // Mouse reporting, focus events…: not yet.
                _ => {}
            }
        }
    }

    /// DSR: 5 = "are you OK?" (yes), 6 = "where is your cursor?".
    fn report(&mut self, question: u16) {
        match question {
            5 => self.reply("\x1b[0n"),
            6 => {
                let row = if self.origin {
                    self.cursor.row.saturating_sub(self.top)
                } else {
                    self.cursor.row
                };
                let answer = format!("\x1b[{};{}R", row + 1, self.cursor.col + 1);
                self.reply(&answer);
            }
            _ => {}
        }
    }

    fn reply(&mut self, answer: &str) {
        if self.replies.len() + answer.len() > MAX_REPLY_BYTES {
            return;
        }
        self.replies.extend_from_slice(answer.as_bytes());
    }
}
