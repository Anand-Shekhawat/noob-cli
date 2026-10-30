//! Runs one line of input the same way for the terminal and for the window:
//! understand it, check it, ask first if it's risky, do it, and describe what happened.
//!
//! Nothing here prints. It hands back `Line`s, and each program shows them its own
//! way: the terminal in colour, the window as a chat.

use crate::actions::{self, Reply};
use crate::command::Command;
use crate::parser::{self, ParseError, Token};

/// Where Noob is being used. Moving around (`go into`) only makes sense where Noob
/// stays open between lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// One phrase typed after `noob` in a normal terminal.
    Terminal,
    /// The `noob>` prompt.
    Prompt,
    /// Noob's own window.
    Window,
}

/// One piece of an answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Line {
    /// Something was done. Shown in green, with a ✓.
    Done(String),
    /// Something to look at, shown as it is.
    Show(String),
    /// Something went wrong. Shown in red, with a ✗.
    Problem(String),
    /// Noob didn't understand, or has a tip. Shown in yellow.
    Hint(String),
    /// The real terminal command that does the same job.
    Real(String),
}

/// What running a line led to.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    pub lines: Vec<Line>,
    pub failed: bool,
    /// The person said bye.
    pub quit: bool,
}

impl Outcome {
    fn failure(line: Line) -> Outcome {
        Outcome {
            lines: vec![line],
            failed: true,
            quit: false,
        }
    }
}

/// What to do next with a line of input.
pub enum Next {
    /// All done: show these lines.
    Finished(Outcome),
    /// Ask this yes/no question first. On "yes", call `finish`; on "no", `declined`.
    Ask { question: String, command: Command },
    /// Ready to go: call `finish`.
    Run(Command),
}

/// Understands a line typed in the `noob>` prompt or the window.
pub fn understand(input: &str, place: Place) -> Next {
    check(parser::parse(input), input.trim(), place)
}

/// Understands the words typed after `noob` in a normal terminal.
/// The shell has already removed any quotes, so a word with a space inside
/// (from `noob make folder "my photos"`) must have been one quoted name.
pub fn understand_words(words: &[String]) -> Next {
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
    check(parsed, &typed, Place::Terminal)
}

fn check(parsed: Result<Command, ParseError>, typed: &str, place: Place) -> Next {
    let command = match parsed {
        Ok(command) => command,
        // Pressing Enter on nothing is fine.
        Err(ParseError::Empty) => return Next::Finished(Outcome::default()),
        Err(error) => return Next::Finished(Outcome::failure(Line::Hint(explain(&error, typed)))),
    };
    if command.prompt_only() && place == Place::Terminal {
        return Next::Finished(Outcome::failure(Line::Hint(format!(
            "? Moving around only works inside the noob prompt. \
             Type noob on its own to open it, then try: {typed}"
        ))));
    }
    if let Err(error) = actions::check_ready(&command) {
        return Next::Finished(Outcome::failure(Line::Problem(error.to_string())));
    }
    match actions::question(&command) {
        Some(question) => Next::Ask { question, command },
        None => Next::Run(command),
    }
}

/// Runs a command. `ask` puts a yes/no question on screen, for commands that ask in
/// the middle of their work (like `install`).
pub fn finish(command: &Command, ask: &mut dyn FnMut(&str) -> bool) -> Outcome {
    // Worked out before running: after a delete or move, the thing it describes is gone.
    let real = command.real_command();
    let mut lines = vec![match actions::execute(command, ask) {
        Ok(Reply::Done(message)) => Line::Done(message),
        Ok(Reply::Show(text)) => Line::Show(text),
        Err(error) => return Outcome::failure(Line::Problem(error.to_string())),
    }];
    lines.extend(real.map(Line::Real));
    Outcome {
        lines,
        failed: false,
        quit: *command == Command::Bye,
    }
}

/// What to say after "no" to an "are you sure?" question.
pub fn declined() -> Outcome {
    Outcome {
        lines: vec![Line::Show("Okay, I left everything as it was.".to_string())],
        ..Outcome::default()
    }
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
        } => format!(
            "? I don't know \"{input}\". Did you mean \"{phrase}\"? Type help to see everything."
        ),
        ParseError::Unknown { suggestion: None } => {
            format!("? I don't know \"{input}\". Type help to see everything I understand.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(next: Next) -> Outcome {
        match next {
            Next::Finished(outcome) => outcome,
            Next::Ask { .. } => panic!("expected it to finish, but it asked"),
            Next::Run(command) => {
                panic!("expected it to finish, but it's ready to run {command:?}")
            }
        }
    }

    #[test]
    fn pressing_enter_on_nothing_does_nothing() {
        assert_eq!(
            finished(understand("   ", Place::Prompt)),
            Outcome::default()
        );
    }

    #[test]
    fn typo_messages_include_the_suggestion() {
        let outcome = finished(understand("shwo files", Place::Prompt));
        assert!(outcome.failed);
        assert!(
            matches!(&outcome.lines[0], Line::Hint(text) if text.contains("Did you mean \"show files\"?"))
        );
    }

    #[test]
    fn moving_around_works_in_the_prompt_and_window_but_not_one_shot() {
        assert!(matches!(
            understand("go back", Place::Prompt),
            Next::Run(Command::GoBack)
        ));
        assert!(matches!(
            understand("go back", Place::Window),
            Next::Run(Command::GoBack)
        ));
        let words = vec!["go".to_string(), "back".to_string()];
        assert!(finished(understand_words(&words)).failed);
    }

    #[test]
    fn risky_commands_ask_first() {
        // Cargo.toml is always there when tests run, so the delete check passes.
        match understand("delete Cargo.toml", Place::Window) {
            Next::Ask { question, command } => {
                assert!(question.contains("Cargo.toml"));
                assert_eq!(command, Command::Delete("Cargo.toml".to_string()));
            }
            _ => panic!("delete must ask first"),
        }
        assert_eq!(
            declined().lines,
            vec![Line::Show("Okay, I left everything as it was.".to_string())]
        );
    }

    #[test]
    fn finishing_gives_the_answer_then_the_real_command() {
        let outcome = finish(&Command::WhereAmI, &mut |_| false);
        assert!(matches!(&outcome.lines[0], Line::Show(text) if text.starts_with("You are in")));
        assert!(matches!(&outcome.lines[1], Line::Real(real) if real == "pwd"));
        assert!(finish(&Command::Bye, &mut |_| false).quit);
    }
}
