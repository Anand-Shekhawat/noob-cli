//! Turning a row of cells into "runs": pieces of text that are drawn in one go.
//!
//! Drawing every cell on its own would be slow, and drawing a whole row as one
//! piece of text breaks as soon as the colours change. So we group cells that
//! sit next to each other and look the same. Each run remembers the column it
//! starts at, so it lands exactly on the grid.
//!
//! Letters outside plain ASCII (accents, symbols, emoji, Chinese…) may come
//! from a fallback font whose letters aren't exactly one cell wide. Each of
//! those gets a run of its own, so it can't push the letters after it sideways.

use noob_cli::term::grid::{Cell, Look, Width};

/// A piece of a row drawn in one go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The column the run starts at (0 is the left edge).
    pub col: usize,
    /// How many columns it covers (a wide letter covers 2).
    pub cols: usize,
    pub text: String,
    pub look: Look,
}

/// Splits a row into runs, left to right. Every column is covered by exactly
/// one run (the right halves of wide letters belong to the letter's run).
pub fn row_runs(cells: &[Cell]) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (col, cell) in cells.iter().enumerate() {
        match cell.width {
            // Already counted with the wide letter on its left.
            Width::Covered => continue,
            Width::Two => runs.push(Run {
                col,
                cols: 2,
                text: cell.letter.to_string(),
                look: cell.look,
            }),
            Width::One => add_letter(&mut runs, col, cell),
        }
    }
    runs
}

/// Adds a one-column letter, joining the previous run when that's safe.
fn add_letter(runs: &mut Vec<Run>, col: usize, cell: &Cell) {
    let joinable = |run: &Run| {
        cell.letter.is_ascii()
            && run.look == cell.look
            && run.col + run.cols == col
            && run.text.is_ascii()
    };
    match runs.last_mut() {
        Some(run) if joinable(run) => {
            run.text.push(cell.letter);
            run.cols += 1;
        }
        _ => runs.push(Run {
            col,
            cols: 1,
            text: cell.letter.to_string(),
            look: cell.look,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noob_cli::term::grid::Color;

    fn plain(letter: char) -> Cell {
        Cell {
            letter,
            look: Look::default(),
            width: Width::One,
        }
    }

    fn red(letter: char) -> Cell {
        Cell {
            look: Look {
                fg: Color::Indexed(1),
                ..Look::default()
            },
            ..plain(letter)
        }
    }

    fn texts(runs: &[Run]) -> Vec<(usize, usize, &str)> {
        runs.iter()
            .map(|run| (run.col, run.cols, run.text.as_str()))
            .collect()
    }

    #[test]
    fn letters_that_look_the_same_are_drawn_together() {
        let row: Vec<Cell> = "ls -la".chars().map(plain).collect();
        assert_eq!(texts(&row_runs(&row)), vec![(0, 6, "ls -la")]);
    }

    #[test]
    fn a_change_of_colour_starts_a_new_run() {
        let row = vec![plain('a'), plain('b'), red('c'), red('d'), plain('e')];
        assert_eq!(
            texts(&row_runs(&row)),
            vec![(0, 2, "ab"), (2, 2, "cd"), (4, 1, "e")]
        );
    }

    #[test]
    fn a_wide_letter_takes_two_columns_and_its_own_run() {
        let wide = Cell {
            letter: '日',
            look: Look::default(),
            width: Width::Two,
        };
        let covered = Cell {
            letter: ' ',
            look: Look::default(),
            width: Width::Covered,
        };
        let row = vec![plain('a'), wide, covered, plain('b')];
        assert_eq!(
            texts(&row_runs(&row)),
            vec![(0, 1, "a"), (1, 2, "日"), (3, 1, "b")]
        );
    }

    #[test]
    fn letters_outside_ascii_stand_alone_so_they_stay_on_the_grid() {
        let row: Vec<Cell> = "a─b".chars().map(plain).collect();
        assert_eq!(
            texts(&row_runs(&row)),
            vec![(0, 1, "a"), (1, 1, "─"), (2, 1, "b")]
        );
    }

    #[test]
    fn every_column_is_covered_exactly_once() {
        let row: Vec<Cell> = "hello world".chars().map(plain).collect();
        let covered: usize = row_runs(&row).iter().map(|run| run.cols).sum();
        assert_eq!(covered, row.len());
    }

    #[test]
    fn an_empty_row_has_no_runs() {
        assert!(row_runs(&[]).is_empty());
    }
}
