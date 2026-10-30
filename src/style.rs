//! Colours for the terminal.
//!
//! Colours are special "escape codes" mixed into the text. They switch off when the
//! output isn't a terminal (saved to a file, or read by another program), or when
//! the `NO_COLOR` setting is present (see no-color.org). Normal output and error
//! messages are checked separately, because they can go to different places
//! (`noob show files 2> errors.txt` sends only the errors to a file).

use std::io::{self, IsTerminal};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

/// Noob's window draws its own colours, so it switches these codes off.
static TURNED_OFF: AtomicBool = AtomicBool::new(false);

pub fn turn_off() {
    TURNED_OFF.store(true, Ordering::Relaxed);
}

fn colour_allowed() -> bool {
    let no_color = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
    !cfg!(test) && !no_color && understands_colour()
}

/// Mac and Linux terminals all understand colour codes. On Windows, Windows Terminal
/// and VS Code do, but the old console window shows them as junk like "[32m".
fn understands_colour() -> bool {
    if !cfg!(windows) {
        return true;
    }
    std::env::var_os("WT_SESSION").is_some() || std::env::var_os("TERM_PROGRAM").is_some()
}

/// `OnceLock` works the answer out the first time, then remembers it.
fn normal_output_has_colour() -> bool {
    static ANSWER: OnceLock<bool> = OnceLock::new();
    *ANSWER.get_or_init(|| colour_allowed() && io::stdout().is_terminal())
}

fn error_output_has_colour() -> bool {
    static ANSWER: OnceLock<bool> = OnceLock::new();
    *ANSWER.get_or_init(|| colour_allowed() && io::stderr().is_terminal())
}

fn paint(code: &str, text: &str, on: bool) -> String {
    if on && !TURNED_OFF.load(Ordering::Relaxed) {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

// For normal output (`println!`).

pub fn success(text: &str) -> String {
    paint("32", text, normal_output_has_colour()) // green
}

pub fn question(text: &str) -> String {
    paint("33", text, normal_output_has_colour()) // yellow
}

pub fn folder(text: &str) -> String {
    paint("1;34", text, normal_output_has_colour()) // bold blue
}

pub fn quiet(text: &str) -> String {
    paint("2", text, normal_output_has_colour()) // dim
}

pub fn prompt(text: &str) -> String {
    paint("36", text, normal_output_has_colour()) // cyan
}

// For error output (`eprintln!`).

pub fn error(text: &str) -> String {
    paint("31", text, error_output_has_colour()) // red
}

pub fn warning(text: &str) -> String {
    paint("33", text, error_output_has_colour()) // yellow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tests_never_see_colour_codes() {
        assert_eq!(success("done"), "done");
        assert_eq!(error("oops"), "oops");
    }
}
