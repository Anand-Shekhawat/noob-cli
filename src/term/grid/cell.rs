//! The little boxes the screen is made of.
//!
//! A terminal screen is a grid: so many rows, so many columns. Each box (a
//! "cell") holds one letter and its *look*: colours and styles like bold.

/// A colour, the way programs ask for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Color {
    /// "Whatever the terminal's normal colour is" (the theme decides).
    #[default]
    Default,
    /// One of the 256 numbered colours. 0-7 are the classic eight (black, red,
    /// green, yellow, blue, magenta, cyan, white), 8-15 their bright versions,
    /// 16-231 a 6×6×6 colour cube and 232-255 greys. The window (stage 4) turns
    /// the number into a real colour, so themes can change the first 16.
    Indexed(u8),
    /// An exact red, green, blue colour ("true colour"), each 0-255.
    Rgb(u8, u8, u8),
}

/// How a letter looks: its colours and styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Look {
    /// The letter's colour ("foreground").
    pub fg: Color,
    /// The colour behind the letter ("background").
    pub bg: Color,
    pub bold: bool,
    /// Fainter than normal.
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    /// Swap the letter and background colours (used for selections and bars).
    pub inverse: bool,
}

/// How much room a cell's letter takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Width {
    /// A normal letter: one cell.
    #[default]
    One,
    /// The left half of a wide letter (emoji, Chinese, Japanese, Korean). The
    /// letter is stored here; the cell to the right is `Covered`.
    Two,
    /// The right half of a wide letter: nothing of its own to show.
    Covered,
}

/// One box on the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub letter: char,
    pub look: Look,
    pub width: Width,
}

impl Cell {
    /// An empty cell. It still has a look, because erasing paints the current
    /// background colour (that's how programs draw coloured bars).
    pub fn blank(look: Look) -> Cell {
        Cell {
            letter: ' ',
            look,
            width: Width::One,
        }
    }
}

impl Default for Cell {
    fn default() -> Cell {
        Cell::blank(Look::default())
    }
}

/// One row of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub cells: Vec<Cell>,
    /// True when the text ran off the end of this row and carried on in the next
    /// one. Copying text (stage 4) uses it to join the two rows back together.
    pub wrapped: bool,
}

impl Row {
    /// A row of `cols` empty cells.
    pub fn blank(cols: usize, look: Look) -> Row {
        Row {
            cells: vec![Cell::blank(look); cols],
            wrapped: false,
        }
    }

    /// The row as plain text, without the spaces at the end.
    pub fn text(&self) -> String {
        let text: String = self
            .cells
            .iter()
            .filter(|cell| cell.width != Width::Covered)
            .map(|cell| cell.letter)
            .collect();
        text.trim_end().to_string()
    }

    /// Makes the row exactly `cols` wide: cut cells off or add empty ones.
    pub(super) fn set_width(&mut self, cols: usize) {
        self.cells.resize(cols, Cell::default());
        self.repair_wide(0, cols);
    }

    /// Fixes broken wide letters between columns `from` and `to`.
    ///
    /// A wide letter is two cells that belong together. When something writes
    /// over one half, erases it, or shifts the row, the other half is left
    /// alone and would show garbage. Real terminals blank the leftover half, and
    /// so do we.
    pub(super) fn repair_wide(&mut self, from: usize, to: usize) {
        let cols = self.cells.len();
        for col in from..to.min(cols) {
            let broken = match self.cells[col].width {
                Width::One => false,
                Width::Two => col + 1 >= cols || self.cells[col + 1].width != Width::Covered,
                Width::Covered => col == 0 || self.cells[col - 1].width != Width::Two,
            };
            if broken {
                let look = self.cells[col].look;
                self.cells[col] = Cell::blank(look);
            }
        }
    }
}
