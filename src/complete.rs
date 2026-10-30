//! Tab completion: what could come next in a half-typed line.
//!
//! At the start of a line it offers phrases ("sh" → "show files"); after a phrase,
//! it offers file names, putting "quotes" around names with spaces.
//! The terminal's prompt and Noob's window both use this.

use std::fs;

use crate::table::{self, COMMANDS, Spec};

/// One Tab choice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    /// What the list of choices shows ("photos/").
    pub display: String,
    /// What goes into the line ("photos/" or "\"my photos/\"").
    pub replacement: String,
}

/// Choices for the end of `typed`, and where in `typed` the choice replaces text.
pub fn complete(typed: &str) -> (usize, Vec<Choice>) {
    let phrases = complete_phrase(typed);
    if !phrases.is_empty() {
        return (0, phrases);
    }
    complete_name(typed)
}

/// "sh" + Tab offers "show files". An empty line + Tab lists every command.
pub fn complete_phrase(typed: &str) -> Vec<Choice> {
    let typed = typed.trim_start().to_lowercase();
    let pool: Vec<&str> = if typed.is_empty() {
        COMMANDS.iter().map(Spec::main_phrase).collect()
    } else {
        table::all_phrases().collect()
    };
    pool.into_iter()
        .filter(|phrase| phrase.starts_with(&typed) && !phrase.starts_with('-'))
        .map(|phrase| Choice {
            display: phrase.to_string(),
            replacement: format!("{phrase} "),
        })
        .collect()
}

/// "read no" + Tab offers "read notes.txt". Names with spaces come back "in quotes".
/// Returns where the name starts in the line, and the choices.
pub fn complete_name(typed: &str) -> (usize, Vec<Choice>) {
    // An odd number of quotes means we're inside "a quoted name".
    let in_quotes = typed.matches('"').count() % 2 == 1;
    let start = if in_quotes {
        typed.rfind('"').unwrap_or(0)
    } else {
        typed.rfind(' ').map_or(0, |space| space + 1)
    };
    let partial = typed[start..].trim_start_matches('"');

    // "photos/ca" means: look inside "photos/" for names starting with "ca".
    let (folder_part, prefix) = match partial.rfind('/') {
        Some(slash) => partial.split_at(slash + 1),
        None => ("", partial),
    };
    let folder = if folder_part.is_empty() {
        "."
    } else {
        folder_part
    };
    let Ok(entries) = fs::read_dir(folder) else {
        return (start, Vec::new()); // a folder we can't look inside has nothing to offer
    };

    let mut choices: Vec<Choice> = entries
        .flatten() // skips any entry we can't read; these are only suggestions
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            // Hidden names (starting with ".") only show up if you type the "." yourself.
            let wanted =
                name.starts_with(prefix) && (prefix.starts_with('.') || !name.starts_with('.'));
            wanted.then(|| choice(folder_part, &name, entry.path().is_dir(), in_quotes))
        })
        .collect();
    choices.sort_by(|a, b| a.display.cmp(&b.display));
    (start, choices)
}

/// One Tab choice. Folders end in "/" so you can keep going inside them.
fn choice(folder_part: &str, name: &str, is_folder: bool, in_quotes: bool) -> Choice {
    let full = format!("{folder_part}{name}");
    let ending = if is_folder { "/" } else { "" };
    let needs_quotes = in_quotes || full.contains(char::is_whitespace);
    let replacement = match (needs_quotes, is_folder) {
        (false, false) => format!("{full} "),
        (false, true) => format!("{full}/"),
        (true, false) => format!("\"{full}\" "),
        (true, true) => format!("\"{full}/\""),
    };
    Choice {
        display: format!("{name}{ending}"),
        replacement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replacements(choices: &[Choice]) -> Vec<&str> {
        choices
            .iter()
            .map(|pair| pair.replacement.as_str())
            .collect()
    }

    #[test]
    fn completes_the_start_of_a_phrase() {
        let choices = complete_phrase("go ");
        let offered = replacements(&choices);
        assert!(offered.contains(&"go into "));
        assert!(offered.contains(&"go home "));
        assert!(!offered.contains(&"show files "));
    }

    #[test]
    fn an_empty_line_offers_each_command_once() {
        assert_eq!(complete_phrase("").len(), COMMANDS.len());
    }

    #[test]
    fn hides_flags_like_dash_h() {
        assert!(complete_phrase("-").is_empty());
    }

    #[test]
    fn after_a_phrase_there_is_nothing_more_to_complete() {
        assert!(complete_phrase("make folder ").is_empty());
    }

    #[test]
    fn completes_file_names_inside_a_folder() {
        let folder = std::env::temp_dir().join(format!("noob-complete-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(folder.join("photos")).unwrap();
        fs::write(folder.join("notes.txt"), "").unwrap();
        fs::write(folder.join("my notes.txt"), "").unwrap();
        fs::write(folder.join(".hidden"), "").unwrap();

        let typed = format!("read {}/", folder.display());
        let (start, choices) = complete_name(&typed);
        assert_eq!(start, "read ".len());
        let shown: Vec<&str> = choices.iter().map(|pair| pair.display.as_str()).collect();
        assert_eq!(
            shown,
            vec!["my notes.txt", "notes.txt", "photos/"],
            "hidden files stay hidden"
        );
        let _ = fs::remove_dir_all(&folder);
    }

    #[test]
    fn quotes_names_that_have_spaces() {
        assert_eq!(
            choice("", "notes.txt", false, false).replacement,
            "notes.txt "
        );
        assert_eq!(
            choice("", "my notes.txt", false, false).replacement,
            "\"my notes.txt\" "
        );
        assert_eq!(
            choice("docs/", "a b", true, false).replacement,
            "\"docs/a b/\""
        );
        assert_eq!(choice("", "photos", true, false).display, "photos/");
    }
}
