//! Selecting text with the mouse, and turning the selection into text to copy.
//!
//! Lines are counted across the history and the screen together: line 0 is the
//! oldest line of history, and the screen's top row comes right after the last
//! one. That way a selection stays put while you scroll.

use noob_cli::term::grid::{Row, Width};

/// A spot in the terminal: a line (history + screen) and a column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Spot {
    pub line: usize,
    pub col: usize,
}

/// What's selected: from where the mouse went down to where it is now.
/// Both ends are included.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Spot,
    pub head: Spot,
}

impl Selection {
    /// The two ends, earliest first.
    pub fn ordered(&self) -> (Spot, Spot) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    /// Just a click, nothing dragged yet.
    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }
}

/// The selected text. `line_at` gives the row for a line number.
///
/// Spaces at the end of each line are dropped, and a line that wrapped onto the
/// next one is joined to it without a line break, so a long command copies as
/// one line.
pub fn selected_text<'a>(
    selection: &Selection,
    line_at: impl Fn(usize) -> Option<&'a Row>,
) -> String {
    let (start, end) = selection.ordered();
    let mut text = String::new();
    for line in start.line..=end.line {
        let Some(row) = line_at(line) else {
            continue;
        };
        let from = if line == start.line { start.col } else { 0 };
        let to = if line == end.line {
            end.col + 1
        } else {
            row.cells.len()
        };
        let piece: String = row
            .cells
            .iter()
            .take(to)
            .skip(from)
            .filter(|cell| cell.width != Width::Covered)
            .map(|cell| cell.letter)
            .collect();
        let joins_next = row.wrapped && line != end.line;
        if joins_next {
            text.push_str(&piece);
        } else {
            text.push_str(piece.trim_end());
            if line != end.line {
                text.push('\n');
            }
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use noob_cli::term::grid::{Cell, Look};

    fn row(text: &str, wrapped: bool) -> Row {
        Row {
            cells: text
                .chars()
                .map(|letter| Cell {
                    letter,
                    look: Look::default(),
                    width: Width::One,
                })
                .collect(),
            wrapped,
        }
    }

    fn select(from: (usize, usize), to: (usize, usize)) -> Selection {
        Selection {
            anchor: Spot {
                line: from.0,
                col: from.1,
            },
            head: Spot {
                line: to.0,
                col: to.1,
            },
        }
    }

    #[test]
    fn part_of_one_line_is_copied_as_is() {
        let rows = [row("hello world ", false)];
        let text = selected_text(&select((0, 6), (0, 10)), |line| rows.get(line));
        assert_eq!(text, "world");
    }

    #[test]
    fn several_lines_keep_their_breaks_and_lose_trailing_spaces() {
        let rows = [row("one   ", false), row("two   ", false)];
        let text = selected_text(&select((0, 0), (1, 5)), |line| rows.get(line));
        assert_eq!(text, "one\ntwo");
    }

    #[test]
    fn a_wrapped_line_joins_the_next_one() {
        let rows = [row("echo ver", true), row("y long  ", false)];
        let text = selected_text(&select((0, 0), (1, 7)), |line| rows.get(line));
        assert_eq!(text, "echo very long");
    }

    #[test]
    fn dragging_backwards_selects_the_same_text() {
        let rows = [row("abc", false), row("def", false)];
        let forwards = selected_text(&select((0, 1), (1, 1)), |line| rows.get(line));
        let backwards = selected_text(&select((1, 1), (0, 1)), |line| rows.get(line));
        assert_eq!(forwards, "bc\nde");
        assert_eq!(forwards, backwards);
    }

    #[test]
    fn a_click_without_dragging_is_empty() {
        assert!(select((1, 1), (1, 1)).is_empty());
        assert!(!select((1, 1), (1, 2)).is_empty());
    }
}
