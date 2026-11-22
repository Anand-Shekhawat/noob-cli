//! Turning the grid's colours into real colours on screen.
//!
//! Programs ask for colours by number ("colour 1", which means red) or exactly
//! ("red 255, green 128, blue 0"). The first 16 numbers are up to the theme,
//! which is how a terminal can look dark or light. The rest are fixed.

use eframe::egui::Color32;
use noob_cli::term::grid::{Color, Look};

/// The colours a terminal is drawn with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub fg: Color32,
    pub bg: Color32,
    /// The bar above the terminal: a shade off the background, so they belong together.
    pub bar: Color32,
    pub cursor: Color32,
    pub selection: Color32,
    /// Black, red, green, yellow, blue, magenta, cyan, white, then the bright ones.
    pub ansi: [Color32; 16],
}

impl Theme {
    /// Soft light text on a deep blue-grey, easy on the eyes for long sessions.
    pub fn dark() -> Theme {
        Theme {
            fg: rgb(0xd8, 0xdc, 0xe4),
            bg: rgb(0x16, 0x19, 0x20),
            bar: rgb(0x10, 0x12, 0x17),
            cursor: rgb(0xf2, 0xc0, 0x5c),
            selection: Color32::from_rgba_unmultiplied(0x5b, 0x8d, 0xd6, 0x66),
            ansi: [
                rgb(0x2a, 0x2f, 0x3a),
                rgb(0xe0, 0x6c, 0x75),
                rgb(0x98, 0xc3, 0x79),
                rgb(0xe5, 0xc0, 0x7b),
                rgb(0x61, 0xaf, 0xef),
                rgb(0xc6, 0x78, 0xdd),
                rgb(0x56, 0xb6, 0xc2),
                rgb(0xc8, 0xcc, 0xd4),
                rgb(0x5c, 0x63, 0x70),
                rgb(0xff, 0x7a, 0x85),
                rgb(0xb5, 0xe0, 0x90),
                rgb(0xff, 0xd8, 0x8f),
                rgb(0x82, 0xc4, 0xff),
                rgb(0xdd, 0x95, 0xf0),
                rgb(0x7f, 0xd6, 0xe0),
                rgb(0xf4, 0xf6, 0xfa),
            ],
        }
    }

    /// Dark text on warm paper, for people who use their computer in light mode.
    pub fn light() -> Theme {
        Theme {
            fg: rgb(0x2b, 0x2f, 0x36),
            bg: rgb(0xfb, 0xfa, 0xf7),
            bar: rgb(0xef, 0xed, 0xe8),
            cursor: rgb(0xc2, 0x6a, 0x00),
            selection: Color32::from_rgba_unmultiplied(0x3b, 0x7d, 0xd8, 0x50),
            ansi: [
                rgb(0x2b, 0x2f, 0x36),
                rgb(0xc0, 0x2f, 0x3a),
                rgb(0x2f, 0x7d, 0x32),
                rgb(0x9a, 0x6a, 0x00),
                rgb(0x1f, 0x5f, 0xc0),
                rgb(0x8e, 0x3c, 0xa8),
                rgb(0x0f, 0x7b, 0x86),
                rgb(0x8a, 0x8f, 0x98),
                rgb(0x5c, 0x63, 0x70),
                rgb(0xe0, 0x45, 0x50),
                rgb(0x3d, 0x9a, 0x40),
                rgb(0xb8, 0x82, 0x00),
                rgb(0x34, 0x78, 0xe0),
                rgb(0xa8, 0x50, 0xc4),
                rgb(0x1a, 0x95, 0xa2),
                rgb(0x60, 0x66, 0x70),
            ],
        }
    }
}

impl Theme {
    /// White text on pure black, with strong, bright colours: the biggest
    /// difference between letters and background, for easier reading.
    pub fn high_contrast() -> Theme {
        Theme {
            fg: rgb(0xff, 0xff, 0xff),
            bg: rgb(0x00, 0x00, 0x00),
            bar: rgb(0x14, 0x14, 0x14),
            cursor: rgb(0xff, 0xd7, 0x00),
            selection: Color32::from_rgba_unmultiplied(0x40, 0x9c, 0xff, 0x80),
            ansi: [
                rgb(0x3a, 0x3a, 0x3a),
                rgb(0xff, 0x5f, 0x5f),
                rgb(0x5f, 0xff, 0x5f),
                rgb(0xff, 0xff, 0x5f),
                rgb(0x6c, 0xb6, 0xff),
                rgb(0xff, 0x7f, 0xff),
                rgb(0x5f, 0xff, 0xff),
                rgb(0xe8, 0xe8, 0xe8),
                rgb(0xa8, 0xa8, 0xa8),
                rgb(0xff, 0x87, 0x87),
                rgb(0x87, 0xff, 0x87),
                rgb(0xff, 0xff, 0x87),
                rgb(0x8f, 0xc8, 0xff),
                rgb(0xff, 0xa0, 0xff),
                rgb(0x87, 0xff, 0xff),
                rgb(0xff, 0xff, 0xff),
            ],
        }
    }

    /// The theme that suits egui's light or dark look.
    pub fn for_dark_mode(dark: bool) -> Theme {
        if dark { Theme::dark() } else { Theme::light() }
    }
}

const fn rgb(red: u8, green: u8, blue: u8) -> Color32 {
    Color32::from_rgb(red, green, blue)
}

/// The colours one cell is drawn with: `(letter, background)`.
///
/// Bold letters in one of the eight classic colours use the bright version,
/// like most terminals do. Inverse swaps the two. Dim fades the letter.
pub fn cell_colors(look: &Look, theme: &Theme) -> (Color32, Color32) {
    let fg_color = match look.fg {
        Color::Indexed(number) if look.bold && number < 8 => Color::Indexed(number + 8),
        other => other,
    };
    let fg = resolve(fg_color, theme, theme.fg);
    let bg = resolve(look.bg, theme, theme.bg);
    let (fg, bg) = if look.inverse { (bg, fg) } else { (fg, bg) };
    let fg = if look.dim {
        fg.lerp_to_gamma(bg, 0.4)
    } else {
        fg
    };
    (fg, bg)
}

/// One colour, with `default` standing in for "the terminal's normal colour".
pub fn resolve(color: Color, theme: &Theme, default: Color32) -> Color32 {
    match color {
        Color::Default => default,
        Color::Rgb(red, green, blue) => rgb(red, green, blue),
        Color::Indexed(number) => indexed(number, theme),
    }
}

/// The 256 numbered colours: 16 from the theme, a 6×6×6 cube, then 24 greys.
fn indexed(number: u8, theme: &Theme) -> Color32 {
    match number {
        0..=15 => theme.ansi[usize::from(number)],
        16..=231 => {
            let cube = number - 16;
            let level = |step: u8| if step == 0 { 0 } else { 55 + step * 40 };
            rgb(level(cube / 36), level((cube / 6) % 6), level(cube % 6))
        }
        232..=255 => {
            let grey = 8 + (number - 232) * 10;
            rgb(grey, grey, grey)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn look(fg: Color) -> Look {
        Look {
            fg,
            ..Look::default()
        }
    }

    /// How much brighter the brighter colour is (1 = the same, 21 = black on white).
    fn contrast(a: Color32, b: Color32) -> f32 {
        let light = |colour: Color32| {
            let channel = |value: u8| {
                let value = f32::from(value) / 255.0;
                if value <= 0.039_28 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * channel(colour.r())
                + 0.7152 * channel(colour.g())
                + 0.0722 * channel(colour.b())
        };
        let (high, low) = (light(a).max(light(b)), light(a).min(light(b)));
        (high + 0.05) / (low + 0.05)
    }

    #[test]
    fn high_contrast_has_more_contrast_than_the_normal_themes() {
        let strong = Theme::high_contrast();
        let ratio = contrast(strong.fg, strong.bg);
        assert!(ratio > contrast(Theme::dark().fg, Theme::dark().bg));
        assert!(ratio > contrast(Theme::light().fg, Theme::light().bg));
        assert!(ratio >= 7.0, "the strictest reading guideline (WCAG AAA)");
    }

    #[test]
    fn normal_text_is_easy_to_read_in_every_theme() {
        for theme in [Theme::dark(), Theme::light(), Theme::high_contrast()] {
            assert!(
                contrast(theme.fg, theme.bg) >= 4.5,
                "WCAG AA for normal text"
            );
        }
    }

    #[test]
    fn default_colours_come_from_the_theme() {
        let theme = Theme::dark();
        assert_eq!(cell_colors(&Look::default(), &theme), (theme.fg, theme.bg));
    }

    #[test]
    fn the_first_sixteen_numbers_come_from_the_theme() {
        let theme = Theme::dark();
        assert_eq!(
            cell_colors(&look(Color::Indexed(1)), &theme).0,
            theme.ansi[1]
        );
        assert_eq!(
            cell_colors(&look(Color::Indexed(12)), &theme).0,
            theme.ansi[12]
        );
    }

    #[test]
    fn bold_classic_colours_turn_bright() {
        let theme = Theme::dark();
        let bold_red = Look {
            bold: true,
            ..look(Color::Indexed(1))
        };
        assert_eq!(cell_colors(&bold_red, &theme).0, theme.ansi[9]);
    }

    #[test]
    fn inverse_swaps_letter_and_background() {
        let theme = Theme::light();
        let inverse = Look {
            inverse: true,
            ..Look::default()
        };
        assert_eq!(cell_colors(&inverse, &theme), (theme.bg, theme.fg));
    }

    #[test]
    fn the_colour_cube_and_greys_follow_xterm() {
        let theme = Theme::dark();
        assert_eq!(indexed(16, &theme), rgb(0, 0, 0));
        assert_eq!(indexed(196, &theme), rgb(255, 0, 0));
        assert_eq!(indexed(231, &theme), rgb(255, 255, 255));
        assert_eq!(indexed(232, &theme), rgb(8, 8, 8));
        assert_eq!(indexed(255, &theme), rgb(238, 238, 238));
    }

    #[test]
    fn exact_colours_are_used_as_they_are() {
        let theme = Theme::dark();
        assert_eq!(
            cell_colors(&look(Color::Rgb(1, 2, 3)), &theme).0,
            rgb(1, 2, 3)
        );
    }

    #[test]
    fn dim_letters_are_closer_to_the_background() {
        let theme = Theme::dark();
        let dim = Look {
            dim: true,
            ..Look::default()
        };
        assert_ne!(cell_colors(&dim, &theme).0, theme.fg);
    }
}
