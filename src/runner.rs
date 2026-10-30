//! Runs one line of input: understand it, ask if it's risky, do it, print the result.

use std::io::{self, BufRead};

use crate::actions::{self, Reply};
use crate::command::Command;
use crate::parser::{self, ParseError, Token};
use crate::style;

/// What happened, so the caller knows whether to keep going.
pub enum Outcome {
    Done,
    Failed,
    Quit,
}

/// Runs one line typed at the `noob>` prompt.
pub fn run_line(input: &str) -> Outcome {
    run(parser::parse(input), input.trim(), true)
}

/// Runs the words typed after `noob` in the normal terminal.
/// The shell has already removed any quotes, so a word with a space inside
/// (from `noob make folder "my photos"`) must have been one quoted name.
pub fn run_words(words: &[String]) -> Outcome {
    let tokens: Vec<Token> = words
        .iter()
        .map(|word| Token {
            text: word.clone(),
            quoted: word.is_empty() || word.contains(char::is_whitespace),
        })
        .collect();
    let typed = words.join(" ");
    // `noob "make folder" photos` puts the phrase itself in quotes. If that isn't
    // understood, try once more, reading every word as if it was typed in the prompt.
    let parsed = match parser::parse_tokens(&tokens) {
        Err(ParseError::Unknown { suggestion }) => {
            parser::parse(&typed).map_err(|_| ParseError::Unknown { suggestion })
        }
        other => other,
    };
    run(parsed, &typed, false)
}

fn run(parsed: Result<Command, ParseError>, typed: &str, in_prompt: bool) -> Outcome {
    let command = match parsed {
        Ok(command) => command,
        Err(ParseError::Empty) => return Outcome::Done, // pressing Enter on nothing is fine
        Err(error) => {
            eprintln!("{}", style::warning(&explain(&error, typed)));
            return Outcome::Failed;
        }
    };

    if command.prompt_only() && !in_prompt {
        let hint = format!(
            "? Moving around only works inside the noob prompt. \
             Type noob on its own to open it, then try: {typed}"
        );
        eprintln!("{}", style::warning(&hint));
        return Outcome::Failed;
    }
    if let Err(error) = actions::check_ready(&command) {
        print_error(&error.to_string());
        return Outcome::Failed;
    }
    if let Some(question) = actions::question(&command)
        && !ask_yes(&question, &mut io::stdin().lock())
    {
        println!("Okay, I left everything as it was.");
        return Outcome::Done;
    }

    // Worked out before running: after a delete or move, the thing it describes is gone.
    let real = command.real_command();
    let mut ask = |question: &str| ask_yes(question, &mut io::stdin().lock());
    match actions::execute(&command, &mut ask) {
        Ok(Reply::Done(message)) => println!("{}", style::success(&format!("✓ {message}"))),
        Ok(Reply::Show(text)) => println!("{text}"),
        Err(error) => {
            print_error(&error.to_string());
            return Outcome::Failed;
        }
    }
    if let Some(real) = real {
        println!("{}", style::quiet(&format!("  (real command: {real})")));
    }

    if command == Command::Bye {
        Outcome::Quit
    } else {
        Outcome::Done
    }
}

fn print_error(message: &str) {
    eprintln!("{}", style::error(&format!("✗ {message}")));
}

/// A friendly sentence for each way the parser can fail.
fn explain(error: &ParseError, input: &str) -> String {
    match error {
        ParseError::Empty => String::new(),
        ParseError::MissingName { example } => {
            format!("? Almost! Tell me the name too, like: {example}")
        }
        ParseError::UnclosedQuote => {
            "? A quote is missing its partner. Use two, like: make folder \"my photos\"".to_string()
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

    #[test]
    fn typo_messages_include_the_suggestion() {
        let error = ParseError::Unknown {
            suggestion: Some("show files"),
        };
        assert!(explain(&error, "shwo files").contains("Did you mean \"show files\"?"));
    }
}
