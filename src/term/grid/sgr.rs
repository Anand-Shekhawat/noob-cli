//! "Select Graphic Rendition" (SGR): the `ESC[…m` command that sets colours
//! and styles for the letters printed after it.
//!
//! Each number is one instruction, read left to right:
//!
//! | Number         | Meaning                                              |
//! |----------------|------------------------------------------------------|
//! | 0              | back to normal                                       |
//! | 1 2 3 4 7      | bold, dim, italic, underline, inverse                |
//! | 22 23 24 27    | turn those off again (22 = bold and dim)             |
//! | 30-37          | letter colour (black, red … white)                   |
//! | 90-97          | bright letter colour                                 |
//! | 40-47, 100-107 | the same for the background                          |
//! | 39, 49         | normal letter / background colour                    |
//! | 38, 48         | a 256 colour (`38;5;208`) or exact one (`38;2;255;0;0`) |

use super::cell::{Color, Look};

/// Changes `look` the way the numbers of one `ESC[…m` ask.
pub(super) fn apply(look: &mut Look, params: &[Vec<u16>]) {
    // `ESC[m` with no numbers means "back to normal", like `ESC[0m`.
    if params.is_empty() {
        *look = Look::default();
        return;
    }
    let mut index = 0;
    while index < params.len() {
        let parts = &params[index];
        let code = parts.first().copied().unwrap_or(0);
        match code {
            0 => *look = Look::default(),
            1 => look.bold = true,
            2 => look.dim = true,
            3 => look.italic = true,
            // `4:0` means "no underline"; `4`, `4:1`, `4:3` (curly) all mean "underline".
            4 => look.underline = parts.get(1).is_none_or(|style| *style != 0),
            7 => look.inverse = true,
            // 21 is "double underline" in most terminals; we draw a single one.
            21 => look.underline = true,
            22 => {
                look.bold = false;
                look.dim = false;
            }
            23 => look.italic = false,
            24 => look.underline = false,
            27 => look.inverse = false,
            30..=37 => look.fg = Color::Indexed(byte(code - 30)),
            39 => look.fg = Color::Default,
            40..=47 => look.bg = Color::Indexed(byte(code - 40)),
            49 => look.bg = Color::Default,
            90..=97 => look.fg = Color::Indexed(byte(code - 90 + 8)),
            100..=107 => look.bg = Color::Indexed(byte(code - 100 + 8)),
            38 | 48 => {
                let (color, used) = extended_color(parts, &params[index + 1..]);
                index += used;
                if let Some(color) = color {
                    if code == 38 {
                        look.fg = color;
                    } else {
                        look.bg = color;
                    }
                }
            }
            // Blinking, hidden, crossed out, fonts…: not supported yet, so ignored.
            _ => {}
        }
        index += 1;
    }
}

/// Reads a 256 or exact colour after a 38 or 48. Says which colour (if the
/// numbers made sense) and how many of the *following* numbers it used up.
///
/// There are two spellings:
///   - colons, all inside one number: `38:5:208`, `38:2:255:0:0`, or with an
///     empty "colour space" slot, `38:2::255:0:0`;
///   - semicolons, spread over the next numbers: `38;5;208`, `38;2;255;0;0`.
fn extended_color(parts: &[u16], rest: &[Vec<u16>]) -> (Option<Color>, usize) {
    if parts.len() > 1 {
        // The colon spelling: everything is in `parts`, nothing else is used.
        let color = match parts[1] {
            5 => parts.get(2).map(|number| Color::Indexed(byte(*number))),
            2 if parts.len() >= 6 => Some(rgb(&parts[3..6])),
            2 if parts.len() == 5 => Some(rgb(&parts[2..5])),
            _ => None,
        };
        return (color, 0);
    }
    // The semicolon spelling: look at the next numbers.
    let numbers: Vec<u16> = rest
        .iter()
        .map(|parts| parts.first().copied().unwrap_or(0))
        .collect();
    match numbers.first() {
        Some(5) => match numbers.get(1) {
            Some(number) => (Some(Color::Indexed(byte(*number))), 2),
            None => (None, numbers.len()),
        },
        Some(2) if numbers.len() >= 4 => (Some(rgb(&numbers[1..4])), 4),
        // Cut off or unknown: skip what's left, so the leftovers aren't
        // mistaken for other styles (`38;2;1` must not turn on bold).
        _ => (None, numbers.len()),
    }
}

/// Three numbers as an exact colour.
fn rgb(numbers: &[u16]) -> Color {
    Color::Rgb(byte(numbers[0]), byte(numbers[1]), byte(numbers[2]))
}

/// A colour number as a byte. Numbers over 255 make no sense; they become 255.
fn byte(number: u16) -> u8 {
    u8::try_from(number).unwrap_or(u8::MAX)
}
