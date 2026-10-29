//! Turns what you typed (like "make folder photos") into a `Command`.

use crate::command::Command;
use crate::suggest;

/// Why we couldn't understand the input.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Nothing was typed.
    Empty,
    /// The phrase was right but a name was missing, e.g. "make folder".
    MissingName { example: &'static str },
    /// We don't know this phrase. Maybe it was a typo of a phrase we do know.
    Unknown { suggestion: Option<&'static str> },
}

pub fn parse(input: &str) -> Result<Command, ParseError> {
    // `words` keeps names exactly as typed ("Photos").
    // `keys` is a lowercase copy, used only to recognise the phrase ("MAKE" == "make").
    let words: Vec<&str> = input.split_whitespace().collect();
    let lowercase: Vec<String> = words.iter().map(|word| word.to_lowercase()).collect();
    let keys: Vec<&str> = lowercase.iter().map(String::as_str).collect();

    // A "slice pattern": each arm describes the shape of the words.
    // `..` means "any number of words after this", and `|` means "or".
    match keys.as_slice() {
        [] => Err(ParseError::Empty),
        ["where", "am", "i"] => Ok(Command::WhereAmI),
        ["show", "files"] => Ok(Command::ShowFiles),
        ["clear"] => Ok(Command::Clear),
        ["help" | "-h" | "--help"] => Ok(Command::Help),
        ["bye" | "exit" | "quit"] => Ok(Command::Bye),
        ["make"] => Err(ParseError::MissingName {
            example: "make folder photos",
        }),
        ["make", "folder" | "dir", ..] => {
            name_from(&words[2..], "make folder photos").map(Command::MakeFolder)
        }
        ["make", "file", ..] => {
            name_from(&words[2..], "make file notes.txt").map(Command::MakeFile)
        }
        ["read", ..] => name_from(&words[1..], "read notes.txt").map(Command::Read),
        ["delete", ..] => name_from(&words[1..], "delete notes.txt").map(Command::Delete),
        ["copy", ..] => two_names(&words[1..], &["to"], "copy a.txt to b.txt")
            .map(|(from, to)| Command::Copy { from, to }),
        ["move", ..] => two_names(&words[1..], &["into", "to"], "move a.txt into docs")
            .map(|(from, to)| Command::Move { from, to }),
        ["rename", ..] => two_names(&words[1..], &["to"], "rename old.txt to new.txt")
            .map(|(from, to)| Command::Rename { from, to }),
        _ => Err(ParseError::Unknown {
            suggestion: suggest::closest(input),
        }),
    }
}

/// Joins the remaining words into one name, so "my photos" works.
fn name_from(words: &[&str], example: &'static str) -> Result<String, ParseError> {
    if words.is_empty() {
        Err(ParseError::MissingName { example })
    } else {
        Ok(words.join(" "))
    }
}

/// Splits "a.txt to b.txt" around a joining word like "to" into ("a.txt", "b.txt").
fn two_names(
    words: &[&str],
    joiners: &[&str],
    example: &'static str,
) -> Result<(String, String), ParseError> {
    let missing = || ParseError::MissingName { example };

    let joiner_at = words
        .iter()
        .position(|word| joiners.contains(&word.to_lowercase().as_str()))
        .ok_or_else(missing)?;

    let (before, after) = (&words[..joiner_at], &words[joiner_at + 1..]);
    if before.is_empty() || after.is_empty() {
        return Err(missing());
    }
    Ok((before.join(" "), after.join(" ")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> String {
        text.to_string()
    }

    #[test]
    fn make_on_its_own_asks_what_to_make() {
        let expected = Err(ParseError::MissingName {
            example: "make folder photos",
        });
        assert_eq!(parse("make"), expected);
    }

    #[test]
    fn understands_the_usual_help_flags() {
        assert_eq!(parse("--help"), Ok(Command::Help));
        assert_eq!(parse("-h"), Ok(Command::Help));
    }

    #[test]
    fn understands_simple_phrases() {
        assert_eq!(parse("where am i"), Ok(Command::WhereAmI));
        assert_eq!(parse("show files"), Ok(Command::ShowFiles));
        assert_eq!(parse("clear"), Ok(Command::Clear));
        assert_eq!(parse("help"), Ok(Command::Help));
    }

    #[test]
    fn ignores_capital_letters_and_extra_spaces() {
        assert_eq!(parse("  Where   AM i  "), Ok(Command::WhereAmI));
    }

    #[test]
    fn has_several_ways_to_say_goodbye() {
        for word in ["bye", "exit", "quit"] {
            assert_eq!(parse(word), Ok(Command::Bye));
        }
    }

    #[test]
    fn makes_folders_with_folder_or_dir() {
        assert_eq!(
            parse("make folder photos"),
            Ok(Command::MakeFolder(name("photos")))
        );
        assert_eq!(
            parse("make dir photos"),
            Ok(Command::MakeFolder(name("photos")))
        );
    }

    #[test]
    fn keeps_the_names_exactly_as_typed() {
        assert_eq!(
            parse("MAKE FOLDER Photos"),
            Ok(Command::MakeFolder(name("Photos")))
        );
    }

    #[test]
    fn allows_names_with_spaces() {
        assert_eq!(
            parse("make folder my photos"),
            Ok(Command::MakeFolder(name("my photos")))
        );
    }

    #[test]
    fn makes_and_reads_and_deletes_files() {
        assert_eq!(
            parse("make file notes.txt"),
            Ok(Command::MakeFile(name("notes.txt")))
        );
        assert_eq!(
            parse("read notes.txt"),
            Ok(Command::Read(name("notes.txt")))
        );
        assert_eq!(
            parse("delete notes.txt"),
            Ok(Command::Delete(name("notes.txt")))
        );
    }

    #[test]
    fn copies_with_to() {
        let expected = Command::Copy {
            from: name("my notes.txt"),
            to: name("backup.txt"),
        };
        assert_eq!(parse("copy my notes.txt to backup.txt"), Ok(expected));
    }

    #[test]
    fn moves_with_into_or_to() {
        let expected = Command::Move {
            from: name("a.txt"),
            to: name("docs"),
        };
        assert_eq!(parse("move a.txt into docs"), Ok(expected.clone()));
        assert_eq!(parse("move a.txt to docs"), Ok(expected));
    }

    #[test]
    fn renames_with_to() {
        let expected = Command::Rename {
            from: name("old.txt"),
            to: name("new.txt"),
        };
        assert_eq!(parse("rename old.txt to new.txt"), Ok(expected));
    }

    #[test]
    fn empty_input_is_reported() {
        assert_eq!(parse(""), Err(ParseError::Empty));
        assert_eq!(parse("    "), Err(ParseError::Empty));
    }

    #[test]
    fn missing_names_get_an_example() {
        let missing = |example| Err(ParseError::MissingName { example });
        assert_eq!(parse("make folder"), missing("make folder photos"));
        assert_eq!(parse("copy a.txt"), missing("copy a.txt to b.txt"));
        assert_eq!(parse("copy to b.txt"), missing("copy a.txt to b.txt"));
        assert_eq!(
            parse("rename old.txt to"),
            missing("rename old.txt to new.txt")
        );
    }

    #[test]
    fn typos_get_a_suggestion() {
        let unknown = parse("maek folder photos");
        assert_eq!(
            unknown,
            Err(ParseError::Unknown {
                suggestion: Some("make folder")
            })
        );
    }

    #[test]
    fn nonsense_gets_no_suggestion() {
        assert_eq!(
            parse("banana bread"),
            Err(ParseError::Unknown { suggestion: None })
        );
    }
}
