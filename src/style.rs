//! Colours for the terminal.
//!
//! Colours are special "escape codes" mixed into the text. They switch off when the
//! output isn't a terminal (saved to a file, or read by another program), or when
//! the `NO_COLOR` setting is present (see no-color.org).

use std::io::IsTerminal;
use std::sync::OnceLock;

fn enabled() -> bool {
    // `OnceLock` works the answer out the first time, then remembers it.
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        !cfg!(test) && !no_color && std::io::stdout().is_terminal()
    })
}

fn paint(code: &str, text: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

pub fn success(text: &str) -> String {
    paint("32", text) // green
}

pub fn error(text: &str) -> String {
    paint("31", text) // red
}

pub fn warning(text: &str) -> String {
    paint("33", text) // yellow
}

pub fn folder(text: &str) -> String {
    paint("1;34", text) // bold blue
}

pub fn quiet(text: &str) -> String {
    paint("2", text) // dim
}

pub fn prompt(text: &str) -> String {
    paint("36", text) // cyan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_never_see_colour_codes() {
        assert_eq!(success("done"), "done");
        assert_eq!(folder("photos/"), "photos/");
    }
}
