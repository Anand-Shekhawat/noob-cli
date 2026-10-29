//! Turns what you typed (like `make folder photos`) into a `Command`,
//! using the phrases in the command table.

use crate::command::Command;
use crate::suggest;
use crate::table::{self, Shape, Spec};

/// Why we couldn't understand the input.
#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Nothing was typed.
    Empty,
    /// The phrase was right but a name was missing, e.g. "make folder".
    MissingName { example: &'static str },
    /// A `"` was opened but never closed.
    UnclosedQuote,
    /// We don't know this phrase. Maybe it was a typo of a phrase we do know.
    Unknown { suggestion: Option<&'static str> },
}

/// One word, or one "quoted name", from the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub quoted: bool,
}

impl Token {
    /// The lowercase word, if this token can be part of a phrase or a joining word like "to".
    /// Quoted text never can: in `copy "to do.txt" to docs`, only the second "to" joins.
    fn keyword(&self) -> Option<String> {
        (!self.quoted).then(|| self.text.to_lowercase())
    }
}

pub fn parse(input: &str) -> Result<Command, ParseError> {
    parse_tokens(&tokenize(input)?)
}

/// Splits input into words, keeping "quoted names" together as one token.
pub fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    // `peekable` lets us look at the next letter without taking it.
    let mut letters = input.chars().peekable();
    while let Some(&letter) = letters.peek() {
        if letter.is_whitespace() {
            letters.next();
        } else if letter == '"' {
            letters.next(); // skip the opening quote
            let mut token = read_quoted(&mut letters)?;
            // Letters right after the closing quote belong to the same name, like in a
            // real terminal: "my photos"2 is "my photos2".
            token.text.push_str(&read_word(&mut letters).text);
            tokens.push(token);
        } else {
            tokens.push(read_word(&mut letters));
        }
    }
    Ok(tokens)
}

/// Reads up to the closing quote. Everything inside, spaces too, is one name.
fn read_quoted(letters: &mut impl Iterator<Item = char>) -> Result<Token, ParseError> {
    let mut text = String::new();
    for letter in letters {
        if letter == '"' {
            return Ok(Token { text, quoted: true });
        }
        text.push(letter);
    }
    Err(ParseError::UnclosedQuote)
}

/// Reads letters up to the next space.
fn read_word(letters: &mut std::iter::Peekable<std::str::Chars<'_>>) -> Token {
    let mut text = String::new();
    while let Some(letter) = letters.next_if(|letter| !letter.is_whitespace()) {
        text.push(letter);
    }
    Token {
        text,
        quoted: false,
    }
}

/// Turns tokens into a command. `main` uses this directly for words typed after `noob`,
/// because the shell has already split them (and removed any quotes).
pub fn parse_tokens(tokens: &[Token]) -> Result<Command, ParseError> {
    if tokens.is_empty() {
        return Err(ParseError::Empty);
    }
    let Some((spec, used)) = best_match(tokens) else {
        let typed = all_text(tokens);
        return Err(ParseError::Unknown {
            suggestion: suggest::closest(&typed),
        });
    };
    let names = names_for(spec, &tokens[used..])?;
    Ok((spec.build)(names))
}

/// The command-table entry that `input` would run, if any. (Only the table tests need this.)
#[cfg(test)]
pub fn spec_for(input: &str) -> Option<&'static Spec> {
    let tokens = tokenize(input).ok()?;
    best_match(&tokens).map(|(spec, _)| spec)
}

/// Finds the phrase that matches the start of the tokens. If several match,
/// the longest wins, so "show all files" would beat "show".
/// Returns the command and how many tokens the phrase used up.
fn best_match(tokens: &[Token]) -> Option<(&'static Spec, usize)> {
    table::COMMANDS
        .iter()
        .flat_map(|spec| spec.phrases.iter().map(move |phrase| (spec, *phrase)))
        .filter_map(|(spec, phrase)| {
            let words: Vec<&str> = phrase.split_whitespace().collect();
            let matches = words.len() <= tokens.len()
                && words
                    .iter()
                    .zip(tokens)
                    .all(|(word, token)| token.keyword().as_deref() == Some(*word));
            matches.then_some((spec, words.len()))
        })
        .max_by_key(|&(_, used)| used)
}

/// Picks out the names after the phrase, in the shape the command expects.
fn names_for(spec: &Spec, rest: &[Token]) -> Result<Vec<String>, ParseError> {
    let missing = || ParseError::MissingName {
        example: spec.example,
    };
    match spec.shape {
        Shape::Nothing if rest.is_empty() => Ok(Vec::new()),
        // "clear everything": the phrase is right, but the extra words don't belong.
        Shape::Nothing => Err(ParseError::Unknown {
            suggestion: Some(spec.main_phrase()),
        }),
        Shape::One => join(rest).map(|name| vec![name]).ok_or_else(missing),
        Shape::Optional => Ok(join(rest).into_iter().collect()),
        Shape::Two { joiners } => {
            let joiner_at = rest
                .iter()
                // The LAST joining word counts, so `write go to the shop to list.txt`
                // writes "go to the shop". Names after the joiner rarely contain it.
                .rposition(|token| {
                    token
                        .keyword()
                        .is_some_and(|word| joiners.contains(&word.as_str()))
                })
                .ok_or_else(missing)?;
            match (join(&rest[..joiner_at]), join(&rest[joiner_at + 1..])) {
                (Some(first), Some(second)) => Ok(vec![first, second]),
                _ => Err(missing()),
            }
        }
    }
}

/// Joins tokens into one name ("my" + "photos" = "my photos"). `None` if there's nothing.
fn join(tokens: &[Token]) -> Option<String> {
    let name = all_text(tokens);
    (!name.is_empty()).then_some(name)
}

fn all_text(tokens: &[Token]) -> String {
    tokens
        .iter()
        .map(|token| token.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> String {
        text.to_string()
    }

    fn word(text: &str) -> Token {
        Token {
            text: name(text),
            quoted: false,
        }
    }

    fn quoted(text: &str) -> Token {
        Token {
            text: name(text),
            quoted: true,
        }
    }

    #[test]
    fn splits_words_and_quoted_names() {
        let tokens = tokenize(r#"copy "my notes.txt"  to docs"#).unwrap();
        assert_eq!(
            tokens,
            vec![
                word("copy"),
                quoted("my notes.txt"),
                word("to"),
                word("docs")
            ]
        );
    }

    #[test]
    fn reports_a_quote_without_its_partner() {
        assert_eq!(
            tokenize(r#"make folder "my photos"#),
            Err(ParseError::UnclosedQuote)
        );
        assert_eq!(
            parse(r#"make folder "my photos"#),
            Err(ParseError::UnclosedQuote)
        );
    }

    #[test]
    fn understands_simple_phrases() {
        assert_eq!(parse("where am i"), Ok(Command::WhereAmI));
        assert_eq!(parse("show files"), Ok(Command::ShowFiles(None)));
        assert_eq!(parse("clear"), Ok(Command::Clear));
        assert_eq!(parse("help"), Ok(Command::Help(None)));
    }

    #[test]
    fn help_can_be_about_one_command() {
        assert_eq!(parse("help copy"), Ok(Command::Help(Some(name("copy")))));
        assert_eq!(parse("--help"), Ok(Command::Help(None)));
        assert_eq!(parse("-h"), Ok(Command::Help(None)));
    }

    #[test]
    fn ignores_capital_letters_and_extra_spaces() {
        assert_eq!(parse("  Where   AM i  "), Ok(Command::WhereAmI));
    }

    #[test]
    fn has_several_ways_to_say_things() {
        for word in ["bye", "exit", "quit"] {
            assert_eq!(parse(word), Ok(Command::Bye));
        }
        for phrase in ["make folder", "make dir", "new folder", "create folder"] {
            let input = format!("{phrase} photos");
            assert_eq!(
                parse(&input),
                Ok(Command::MakeFolder(name("photos"))),
                "{phrase}"
            );
        }
    }

    #[test]
    fn keeps_the_names_exactly_as_typed() {
        assert_eq!(
            parse("MAKE FOLDER Photos"),
            Ok(Command::MakeFolder(name("Photos")))
        );
    }

    #[test]
    fn allows_names_with_spaces_with_or_without_quotes() {
        let expected = Ok(Command::MakeFolder(name("my photos")));
        assert_eq!(parse("make folder my photos"), expected);
        assert_eq!(parse(r#"make folder "my photos""#), expected);
    }

    #[test]
    fn quoted_names_can_contain_the_joining_word() {
        let expected = Command::Copy {
            from: name("notes to self.txt"),
            to: name("docs"),
        };
        assert_eq!(parse(r#"copy "notes to self.txt" to docs"#), Ok(expected));
    }

    #[test]
    fn text_can_contain_the_joining_word_because_the_last_one_counts() {
        let write = Command::Write {
            text: name("go to the shop"),
            file: name("list.txt"),
        };
        assert_eq!(parse("write go to the shop to list.txt"), Ok(write));
        let copy = Command::Copy {
            from: name("notes to self.txt"),
            to: name("docs"),
        };
        assert_eq!(parse("copy notes to self.txt to docs"), Ok(copy));
    }

    #[test]
    fn a_quote_glued_to_a_word_makes_one_name() {
        assert_eq!(
            tokenize(r#""my photos"2"#).unwrap(),
            vec![quoted("my photos2")]
        );
    }

    #[test]
    fn joining_words_ignore_capitals() {
        let expected = Command::Rename {
            from: name("a.txt"),
            to: name("b.txt"),
        };
        assert_eq!(parse("rename a.txt TO b.txt"), Ok(expected));
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
        assert_eq!(
            parse("remove notes.txt"),
            Ok(Command::Delete(name("notes.txt")))
        );
    }

    #[test]
    fn copies_and_moves_with_to_or_into() {
        let copy = Command::Copy {
            from: name("my notes.txt"),
            to: name("backup.txt"),
        };
        assert_eq!(parse("copy my notes.txt to backup.txt"), Ok(copy));

        let moved = Command::Move {
            from: name("a.txt"),
            to: name("docs"),
        };
        assert_eq!(parse("move a.txt into docs"), Ok(moved.clone()));
        assert_eq!(parse("move a.txt to docs"), Ok(moved));
    }

    #[test]
    fn understands_going_places() {
        assert_eq!(parse("go into photos"), Ok(Command::GoInto(name("photos"))));
        assert_eq!(
            parse("go to my photos"),
            Ok(Command::GoInto(name("my photos")))
        );
        assert_eq!(parse("go back"), Ok(Command::GoBack));
        assert_eq!(parse("go up"), Ok(Command::GoBack));
        assert_eq!(parse("go home"), Ok(Command::GoHome));
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
        assert_eq!(parse(r#"make folder """#), missing("make folder photos"));
        assert_eq!(parse("copy a.txt"), missing("copy a.txt to b.txt"));
        assert_eq!(parse("copy to b.txt"), missing("copy a.txt to b.txt"));
        assert_eq!(
            parse("rename old.txt to"),
            missing("rename old.txt to new.txt")
        );
    }

    #[test]
    fn half_a_phrase_suggests_the_whole_phrase() {
        let suggestion = |phrase| {
            Err(ParseError::Unknown {
                suggestion: Some(phrase),
            })
        };
        assert_eq!(parse("make"), suggestion("make folder"));
        assert_eq!(parse("show"), suggestion("show files"));
    }

    #[test]
    fn extra_words_after_a_complete_phrase_are_questioned() {
        let unknown = Err(ParseError::Unknown {
            suggestion: Some("clear"),
        });
        assert_eq!(parse("clear everything"), unknown);
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

    #[test]
    fn tokens_from_the_shell_keep_spaces_inside_one_name() {
        // `noob make folder "my photos"` reaches us as three words, the last with a space.
        let tokens = vec![word("make"), word("folder"), quoted("my photos")];
        assert_eq!(
            parse_tokens(&tokens),
            Ok(Command::MakeFolder(name("my photos")))
        );
    }
}
