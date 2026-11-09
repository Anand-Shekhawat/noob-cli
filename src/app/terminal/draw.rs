//! Painting the grid: backgrounds, letters, the selection and the cursor.
//!
//! Everything is painted fresh each frame, which is how egui works. It's quick
//! enough because a screen is only a few thousand cells and egui remembers
//! text it has laid out before.

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{Color32, FontId, Painter, Stroke, StrokeKind, pos2, vec2};
use noob_cli::term::grid::{Grid, Row, Width};

use super::layout::Layout;
use super::palette::{Theme, cell_colors};
use super::runs::{Run, row_runs};
use super::select::Selection;

/// Bold is drawn twice, the second time nudged right by this much. The
/// built-in monospace font has no bold version.
const BOLD_NUDGE: f32 = 0.6;

/// How the cursor looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorLook {
    /// A filled block: the window has focus and the blink is "on".
    Block,
    /// An outline: the window isn't focused.
    Outline,
}

/// Everything needed to paint one frame of the terminal.
pub struct Scene<'a> {
    pub grid: &'a Grid,
    pub layout: Layout,
    pub theme: &'a Theme,
    pub font: FontId,
    /// The line (history + screen) shown in the top row.
    pub top_line: usize,
    pub selection: Option<Selection>,
    /// `None` hides the cursor (blink "off", scrolled away, or a program hid it).
    pub cursor: Option<CursorLook>,
}

/// A line counted across history and screen: history first, then the screen.
pub fn line_at(grid: &Grid, line: usize) -> Option<&Row> {
    let history = grid.scrollback_len();
    if line < history {
        grid.scrollback_row(line)
    } else {
        grid.row(line - history)
    }
}

pub fn paint(painter: &Painter, scene: &Scene) {
    for screen_row in 0..scene.layout.rows {
        let Some(row) = line_at(scene.grid, scene.top_line + screen_row) else {
            continue;
        };
        let cells = &row.cells[..row.cells.len().min(scene.layout.cols)];
        for run in row_runs(cells) {
            paint_run(painter, scene, screen_row, &run);
        }
    }
    if let Some(selection) = scene.selection {
        paint_selection(painter, scene, &selection);
    }
    if let Some(look) = scene.cursor {
        paint_cursor(painter, scene, look);
    }
}

fn paint_run(painter: &Painter, scene: &Scene, screen_row: usize, run: &Run) {
    let (fg, bg) = cell_colors(&run.look, scene.theme);
    let rect = scene.layout.cells_rect(screen_row, run.col, run.cols);
    if bg != scene.theme.bg {
        painter.rect_filled(rect, 0.0, bg);
    }
    if run.text.trim().is_empty() && !run.look.underline {
        return;
    }
    let format = TextFormat {
        font_id: scene.font.clone(),
        color: fg,
        italics: run.look.italic,
        underline: if run.look.underline {
            Stroke::new(1.0, fg)
        } else {
            Stroke::NONE
        },
        ..TextFormat::default()
    };
    let galley = painter.layout_job(LayoutJob::single_section(run.text.clone(), format));
    // A lone letter (wide, or from a fallback font) is centred in its cells;
    // plain ASCII starts at the left edge so it lines up with the grid.
    let spare = rect.width() - galley.size().x;
    let x = if run.text.is_ascii() {
        rect.left()
    } else {
        rect.left() + (spare / 2.0).max(0.0)
    };
    let y = rect.top() + (rect.height() - galley.size().y) / 2.0;
    if run.look.bold {
        painter.galley(pos2(x + BOLD_NUDGE, y), galley.clone(), fg);
    }
    painter.galley(pos2(x, y), galley, fg);
}

fn paint_selection(painter: &Painter, scene: &Scene, selection: &Selection) {
    if selection.is_empty() {
        return;
    }
    let (start, end) = selection.ordered();
    for screen_row in 0..scene.layout.rows {
        let line = scene.top_line + screen_row;
        if line < start.line || line > end.line {
            continue;
        }
        let from = if line == start.line { start.col } else { 0 };
        let to = if line == end.line {
            end.col + 1
        } else {
            scene.layout.cols
        };
        if to > from {
            let rect = scene.layout.cells_rect(screen_row, from, to - from);
            painter.rect_filled(rect, 0.0, scene.theme.selection);
        }
    }
}

fn paint_cursor(painter: &Painter, scene: &Scene, look: CursorLook) {
    let (row, mut col) = scene.grid.cursor();
    let mut width = 1;
    if let Some(cell) = scene.grid.cell(row, col) {
        match cell.width {
            Width::Two => width = 2,
            Width::Covered if col > 0 => {
                col -= 1;
                width = 2;
            }
            _ => {}
        }
    }
    let rect = scene.layout.cells_rect(row, col, width);
    match look {
        CursorLook::Outline => {
            painter.rect_stroke(
                rect.shrink(0.5),
                1.0,
                Stroke::new(1.0, scene.theme.cursor),
                StrokeKind::Inside,
            );
        }
        CursorLook::Block => {
            painter.rect_filled(rect, 1.0, scene.theme.cursor);
            // The letter under the cursor, in the background colour so it shows.
            if let Some(cell) = scene.grid.cell(row, col)
                && cell.letter != ' '
            {
                let galley = painter.layout_no_wrap(
                    cell.letter.to_string(),
                    scene.font.clone(),
                    scene.theme.bg,
                );
                let spare = rect.size() - galley.size();
                let x = if cell.letter.is_ascii() {
                    0.0
                } else {
                    (spare.x / 2.0).max(0.0)
                };
                painter.galley(
                    rect.min + vec2(x, spare.y / 2.0),
                    galley,
                    Color32::PLACEHOLDER,
                );
            }
        }
    }
}
