//! Where each cell sits in the window, and how many fit.

use eframe::egui::{Pos2, Rect, Vec2, vec2};

/// Space between the window's edge and the text, in points.
pub const PADDING: f32 = 10.0;

/// The grid laid over a piece of the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Top-left corner of cell (0, 0).
    pub origin: Pos2,
    /// The size of one cell.
    pub cell: Vec2,
    pub rows: usize,
    pub cols: usize,
}

impl Layout {
    /// As many whole cells as fit inside `area` (at least 1 × 1).
    pub fn fit(area: Rect, cell: Vec2) -> Layout {
        let inner = area.shrink(PADDING);
        let count = |space: f32, size: f32| {
            if size <= 0.0 {
                1
            } else {
                ((space / size).floor() as usize).max(1)
            }
        };
        Layout {
            origin: inner.min,
            cell,
            rows: count(inner.height(), cell.y),
            cols: count(inner.width(), cell.x),
        }
    }

    /// The rectangle of `width` cells starting at (`row`, `col`).
    pub fn cells_rect(&self, row: usize, col: usize, width: usize) -> Rect {
        let min = self.origin + vec2(col as f32 * self.cell.x, row as f32 * self.cell.y);
        Rect::from_min_size(min, vec2(width as f32 * self.cell.x, self.cell.y))
    }

    /// The (row, column) under a point, kept inside the grid.
    pub fn cell_at(&self, point: Pos2) -> (usize, usize) {
        let along = |offset: f32, size: f32, count: usize| {
            let index = (offset / size).floor().max(0.0) as usize;
            index.min(count - 1)
        };
        (
            along(point.y - self.origin.y, self.cell.y, self.rows),
            along(point.x - self.origin.x, self.cell.x, self.cols),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::pos2;

    fn area(width: f32, height: f32) -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(width, height))
    }

    #[test]
    fn as_many_whole_cells_as_fit_inside_the_padding() {
        let layout = Layout::fit(area(820.0, 500.0), vec2(8.0, 16.0));
        assert_eq!(layout.cols, 100, "(820 - 2×10) / 8");
        assert_eq!(layout.rows, 30, "(500 - 2×10) / 16");
        assert_eq!(layout.origin, pos2(PADDING, PADDING));
    }

    #[test]
    fn a_tiny_window_still_has_one_cell() {
        let layout = Layout::fit(area(5.0, 5.0), vec2(8.0, 16.0));
        assert_eq!((layout.rows, layout.cols), (1, 1));
    }

    #[test]
    fn points_map_to_cells_and_stay_inside_the_grid() {
        let layout = Layout::fit(area(820.0, 500.0), vec2(8.0, 16.0));
        assert_eq!(layout.cell_at(pos2(10.0, 10.0)), (0, 0));
        assert_eq!(
            layout.cell_at(pos2(10.0 + 8.0 * 3.5, 10.0 + 16.0 * 2.2)),
            (2, 3)
        );
        assert_eq!(layout.cell_at(pos2(-50.0, -50.0)), (0, 0));
        assert_eq!(layout.cell_at(pos2(5000.0, 5000.0)), (29, 99));
    }

    #[test]
    fn a_wide_letter_rectangle_spans_two_cells() {
        let layout = Layout::fit(area(820.0, 500.0), vec2(8.0, 16.0));
        let rect = layout.cells_rect(1, 2, 2);
        assert_eq!(rect.min, pos2(10.0 + 16.0, 10.0 + 16.0));
        assert_eq!(rect.width(), 16.0);
    }
}
