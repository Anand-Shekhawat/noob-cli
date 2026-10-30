//! The terminal's side of running a line: hand it to Noob's engine, answer its
//! "are you sure?" questions from the keyboard, and print what happened in colour.

use std::io::{self, BufRead};

use noob_cli::engine::{self, Line, Next, Place};
use noob_cli::style;

/// What happened, so the caller knows whether to keep going.
pub enum Outcome {
    Done,
    Failed,
    Quit,
}

/// Runs one line typed at the `noob>` prompt.
pub fn run_line(input: &str) -> Outcome {
    handle(engine::understand(input, Place::Prompt))
}

/// Runs the words typed after `noob` in the normal terminal.
pub fn run_words(words: &[String]) -> Outcome {
    handle(engine::understand_words(words))
}

fn handle(next: Next) -> Outcome {
    let mut ask = |question: &str| ask_yes(question, &mut io::stdin().lock());
    let outcome = match next {
        Next::Finished(outcome) => outcome,
        Next::Ask { question, command } => {
            if ask(&question) {
                engine::finish(&command, &mut ask)
            } else {
                engine::declined()
            }
        }
        Next::Run(command) => engine::finish(&command, &mut ask),
    };
    for line in &outcome.lines {
        print(line);
    }
    match (outcome.quit, outcome.failed) {
        (true, _) => Outcome::Quit,
        (false, true) => Outcome::Failed,
        (false, false) => Outcome::Done,
    }
}

fn print(line: &Line) {
    match line {
        Line::Done(message) => println!("{}", style::success(&format!("✓ {message}"))),
        Line::Show(text) => println!("{text}"),
        Line::Problem(message) => print_error(message),
        Line::Hint(message) => eprintln!("{}", style::warning(message)),
        Line::Real(real) => println!("{}", style::quiet(&format!("  (real command: {real})"))),
    }
}

fn print_error(message: &str) {
    eprintln!("{}", style::error(&format!("✗ {message}")));
}

/// Prints the question and returns true only if the user types "yes" or "y".
/// `input` is the keyboard in real use, but tests can pass in any text instead.
fn ask_yes(question: &str, input: &mut impl BufRead) -> bool {
    println!(
        "{}",
        style::question(&format!("⚠ {question} Type yes to continue:"))
    );
    let mut answer = String::new();
    match input.read_line(&mut answer) {
        Ok(0) => false, // input closed without an answer, so the answer is no
        Ok(_) => matches!(answer.trim().to_lowercase().as_str(), "yes" | "y"),
        Err(error) => {
            print_error(&format!(
                "I couldn't read your answer ({error}), so I did nothing."
            ));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_yes_means_yes() {
        // `&[u8]` (bytes) can stand in for the keyboard in tests.
        assert!(ask_yes("Sure?", &mut "yes\n".as_bytes()));
        assert!(ask_yes("Sure?", &mut "Y\n".as_bytes()));
        assert!(!ask_yes("Sure?", &mut "no\n".as_bytes()));
        assert!(!ask_yes("Sure?", &mut "".as_bytes()));
    }
}
