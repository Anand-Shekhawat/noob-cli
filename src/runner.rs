//! Runs one line of input: understand it, ask if it's risky, do it, print the result.

use std::io::{self, BufRead};

use crate::actions;
use crate::command::Command;
use crate::parser::{self, ParseError};

/// What happened, so the caller knows whether to keep going.
pub enum Outcome {
    Done,
    Failed,
    Quit,
}

pub fn run_line(input: &str) -> Outcome {
    let command = match parser::parse(input) {
        Ok(command) => command,
        Err(ParseError::Empty) => return Outcome::Done, // pressing Enter on nothing is fine
        Err(error) => {
            eprintln!("{}", explain(&error, input.trim()));
            return Outcome::Failed;
        }
    };

    if let Err(error) = actions::check_ready(&command) {
        eprintln!("✗ {error}");
        return Outcome::Failed;
    }
    if let Some(question) = command.confirmation_question()
        && !ask_yes(&question, &mut io::stdin().lock())
    {
        println!("Okay, I left everything as it was.");
        return Outcome::Done;
    }

    match actions::execute(&command) {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("✗ {error}");
            return Outcome::Failed;
        }
    }
    if let Some(real) = command.real_command() {
        println!("  (real command: {real})");
    }

    if command == Command::Bye {
        Outcome::Quit
    } else {
        Outcome::Done
    }
}

/// A friendly sentence for each way the parser can fail.
fn explain(error: &ParseError, input: &str) -> String {
    match error {
        ParseError::Empty => String::new(),
        ParseError::MissingName { example } => {
            format!("? Almost! Tell me the name too, like: {example}")
        }
        ParseError::Unknown {
            suggestion: Some(phrase),
        } => {
            format!(
                "? I don't know \"{input}\". Did you mean \"{phrase}\"? Type help to see everything."
            )
        }
        ParseError::Unknown { suggestion: None } => {
            format!("? I don't know \"{input}\". Type help to see everything I understand.")
        }
    }
}

/// Prints the question and returns true only if the user types "yes" or "y".
/// `input` is the keyboard in real use, but tests can pass in any text instead.
fn ask_yes(question: &str, input: &mut impl BufRead) -> bool {
    println!("⚠ {question} Type yes to continue:");
    let mut answer = String::new();
    match input.read_line(&mut answer) {
        Ok(0) => false, // input closed without an answer, so the answer is no
        Ok(_) => matches!(answer.trim().to_lowercase().as_str(), "yes" | "y"),
        Err(error) => {
            eprintln!("✗ I couldn't read your answer ({error}), so I did nothing.");
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

    #[test]
    fn typo_messages_include_the_suggestion() {
        let error = ParseError::Unknown {
            suggestion: Some("show files"),
        };
        assert!(explain(&error, "shwo files").contains("Did you mean \"show files\"?"));
    }
}
