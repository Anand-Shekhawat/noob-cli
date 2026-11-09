//! Making the screen bigger or smaller when the window changes size.
//!
//! What real terminals do, and what we copy:
//!   - Fewer columns: rows are cut off on the right (no re-flowing of long
//!     lines yet). Half a wide letter at the new edge is blanked.
//!   - Fewer rows: the row with the cursor stays on screen. Rows above it that
//!     no longer fit go into the history (on the main screen).
//!   - More rows: lines come back from the history onto the top, so shrinking
//!     and growing again gets you back where you were. Otherwise, empty rows
//!     are added at the bottom.
//!
//! The scroll region goes back to the whole screen; programs that use one
//! (vim, less) set it again after a resize anyway.

use std::collections::VecDeque;

use super::{Grid, Look, MAX_SIZE, Row, default_tabs};

impl Grid {
    /// Changes the screen to `rows` × `cols` (at least 1 × 1).
    pub fn resize(&mut self, rows: usize, cols: usize) {
        let rows = rows.clamp(1, MAX_SIZE);
        let cols = cols.clamp(1, MAX_SIZE);
        if (rows, cols) == (self.rows, self.cols) {
            return;
        }

        for line in self.lines.iter_mut().chain(self.other.iter_mut()) {
            line.set_width(cols);
        }

        // Which row each screen must keep: the cursor's on the shown screen,
        // the saved cursor's on the hidden one (that's where it goes back to).
        let shown_row = self.cursor.row;
        let hidden_row = self.other_saved.map_or(shown_row, |saved| saved.cursor.row);
        let limit = self.scrollback_limit;
        let (shown_history, hidden_history) = if self.alternate {
            (None, Some(&mut self.scrollback))
        } else {
            (Some(&mut self.scrollback), None)
        };
        let shown_shift = fit_rows(&mut self.lines, shown_row, rows, cols, shown_history, limit);
        let hidden_shift = fit_rows(
            &mut self.other,
            hidden_row,
            rows,
            cols,
            hidden_history,
            limit,
        );

        self.rows = rows;
        self.cols = cols;
        self.top = 0;
        self.bottom = rows - 1;
        let mut tabs = default_tabs(cols);
        let kept = self.tabs.len().min(cols);
        tabs[..kept].copy_from_slice(&self.tabs[..kept]);
        self.tabs = tabs;

        self.cursor.row = shift_row(self.cursor.row, shown_shift, rows);
        self.cursor.col = self.cursor.col.min(cols - 1);
        self.cursor.pending_wrap = false;
        for (saved, shift) in [
            (&mut self.saved, shown_shift),
            (&mut self.other_saved, hidden_shift),
        ] {
            if let Some(saved) = saved {
                saved.cursor.row = shift_row(saved.cursor.row, shift, rows);
                saved.cursor.col = saved.cursor.col.min(cols - 1);
            }
        }
    }
}

/// Makes one screen exactly `rows` tall, keeping row `keep` on it. Lines that
/// leave the top go into `history` (if this screen has one), and lines come
/// back from it when there's room. Says how far the old rows moved down
/// (negative: up), so the cursor can move with them.
fn fit_rows(
    lines: &mut Vec<Row>,
    keep: usize,
    rows: usize,
    cols: usize,
    mut history: Option<&mut VecDeque<Row>>,
    limit: usize,
) -> isize {
    let old = lines.len();
    if rows < old {
        let off_the_top = (keep + 1).saturating_sub(rows);
        for line in lines.drain(..off_the_top) {
            if let Some(history) = history.as_deref_mut()
                && limit > 0
            {
                if history.len() >= limit {
                    history.pop_front();
                }
                history.push_back(line);
            }
        }
        lines.truncate(rows);
        return -to_signed(off_the_top);
    }

    let mut pulled = 0;
    if let Some(history) = history {
        while lines.len() < rows {
            let Some(mut line) = history.pop_back() else {
                break;
            };
            line.set_width(cols);
            lines.insert(0, line);
            pulled += 1;
        }
    }
    lines.resize(rows, Row::blank(cols, Look::default()));
    to_signed(pulled)
}

/// Moves a row number by `shift`, keeping it on a screen of `rows` rows.
fn shift_row(row: usize, shift: isize, rows: usize) -> usize {
    row.saturating_add_signed(shift).min(rows - 1)
}

/// Row counts are at most `MAX_SIZE`, so they always fit in an `isize`.
fn to_signed(count: usize) -> isize {
    isize::try_from(count).unwrap_or(isize::MAX)
}
