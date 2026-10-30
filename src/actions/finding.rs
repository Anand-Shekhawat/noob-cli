//! Finding things: `find file cat` and `find "hello" in notes.txt`.

use std::fs;
use std::path::Path;

use super::Reply;
use super::errors::{ActionError, friendly};
use super::paths::existing;
use super::walk::{self, Next, Summary};
use super::words::plural;
use crate::style;

const MAX_NAME_RESULTS: usize = 50;
const MAX_TEXT_RESULTS: usize = 100;
/// Bigger files are skipped when searching a whole folder, to keep searches quick.
const MAX_SEARCH_FILE_SIZE: u64 = 5 * 1024 * 1024;
/// Long lines are cut short so one line can't fill the screen.
const MAX_LINE_LENGTH: usize = 100;

pub(super) fn find_file(name: &str) -> Result<Reply, ActionError> {
    find_name_in(Path::new("."), name)
}

/// Looks for names containing `name`, ignoring capital letters, in `root` and every folder inside.
fn find_name_in(root: &Path, name: &str) -> Result<Reply, ActionError> {
    let wanted = name.to_lowercase();
    let mut found: Vec<(String, bool)> = Vec::new();
    let summary = walk::walk(root, false, |path, metadata| {
        let matches = path
            .file_name()
            .is_some_and(|file_name| file_name.to_string_lossy().to_lowercase().contains(&wanted));
        if matches {
            found.push((relative(root, path), metadata.is_dir()));
        }
        if found.len() >= MAX_NAME_RESULTS {
            Next::Stop
        } else {
            Next::Continue
        }
    });

    if found.is_empty() {
        return Ok(Reply::Show(format!(
            "I couldn't find anything called \"{name}\" here, or in the folders inside.{}",
            summary.note()
        )));
    }
    found.sort();
    let mut lines = vec![format!(
        "Found {} called \"{name}\":",
        plural(found.len() as u64, "thing")
    )];
    lines.extend(found.iter().map(|(path, is_folder)| {
        if *is_folder {
            format!("  📁 {}", style::folder(&format!("{path}/")))
        } else {
            format!("  📄 {path}")
        }
    }));
    if summary.stopped {
        lines.push(format!(
            "… I stopped after {MAX_NAME_RESULTS}. Try a longer name."
        ));
    }
    Ok(Reply::Show(lines.join("\n") + &summary.note()))
}

/// Looks for `text` inside one file, or inside every text file in a folder.
pub(super) fn find_text(text: &str, place: &str) -> Result<Reply, ActionError> {
    let path = existing(place)?;
    let wanted = text.to_lowercase();
    let mut matches: Vec<String> = Vec::new();
    let mut skipped: u64 = 0;

    let summary = if path.is_dir() {
        walk::walk(path, false, |file, metadata| {
            if metadata.is_file() {
                // Big files and files that aren't text (pictures, programs) are skipped,
                // and counted so we can say so.
                let content = (metadata.len() <= MAX_SEARCH_FILE_SIZE)
                    .then(|| fs::read_to_string(file).ok())
                    .flatten();
                match content {
                    Some(content) => {
                        let label = relative(Path::new("."), file);
                        search_lines(&content, &label, &wanted, &mut matches);
                    }
                    None => skipped += 1,
                }
            }
            if matches.len() >= MAX_TEXT_RESULTS {
                Next::Stop
            } else {
                Next::Continue
            }
        })
    } else {
        let info = fs::metadata(path).map_err(|error| friendly(error, place))?;
        if !info.is_file() {
            return Err(ActionError::NotAFile(place.to_string()));
        }
        let content = fs::read_to_string(path).map_err(|error| friendly(error, place))?;
        search_lines(&content, place, &wanted, &mut matches);
        Summary::default()
    };

    let skipped_note = match skipped {
        0 => String::new(),
        count => format!(
            "\n(I skipped {}: not text, or very big.)",
            plural(count, "file")
        ),
    };
    Ok(Reply::Show(
        describe_matches(text, place, &matches, &summary) + &skipped_note,
    ))
}

fn search_lines(content: &str, label: &str, wanted: &str, matches: &mut Vec<String>) {
    for (index, line) in content.lines().enumerate() {
        if matches.len() >= MAX_TEXT_RESULTS {
            return;
        }
        if line.to_lowercase().contains(wanted) {
            let place = style::quiet(&format!("{label}:{}", index + 1)); // lines count from 1
            matches.push(format!("  {place}  {}", shorten(line.trim())));
        }
    }
}

fn describe_matches(text: &str, place: &str, matches: &[String], summary: &Summary) -> String {
    if matches.is_empty() {
        return format!("I couldn't find \"{text}\" in {place}.{}", summary.note());
    }
    let times = match matches.len() {
        1 => "once".to_string(),
        count => format!("{count} times"),
    };
    let mut lines = vec![format!("Found \"{text}\" {times}:")];
    lines.extend(matches.iter().cloned());
    if matches.len() >= MAX_TEXT_RESULTS {
        lines.push(format!("… I stopped after {MAX_TEXT_RESULTS}."));
    }
    lines.join("\n") + &summary.note()
}

/// `path` written from inside `root`: "./photos/cat.jpg" becomes "photos/cat.jpg".
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn shorten(line: &str) -> String {
    if line.chars().count() <= MAX_LINE_LENGTH {
        return line.to_string();
    }
    let start: String = line.chars().take(MAX_LINE_LENGTH).collect();
    format!("{start}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::test_support::TestFolder;

    fn shown(reply: Result<Reply, ActionError>) -> String {
        match reply {
            Ok(Reply::Show(text)) => text,
            other => panic!("expected something to show, got {other:?}"),
        }
    }

    #[test]
    fn finds_names_in_folders_inside_ignoring_capitals() {
        let test = TestFolder::new("find-name");
        fs::create_dir_all(test.at("pets/cats")).unwrap();
        fs::write(test.at("pets/cats/Cat.jpg"), "").unwrap();
        fs::write(test.at(".cat-secret"), "").unwrap();

        let result = shown(find_name_in(test.dir(), "cat"));
        assert!(
            result.starts_with("Found 2 things called \"cat\":"),
            "{result}"
        );
        let cats = Path::new("pets").join("cats");
        assert!(
            result.contains(&format!("📁 {}/", cats.display())),
            "{result}"
        );
        assert!(result.contains(&format!("📄 {}", cats.join("Cat.jpg").display())));
        assert!(!result.contains("secret"), "hidden files stay hidden");
    }

    #[test]
    fn says_so_when_nothing_has_that_name() {
        let test = TestFolder::new("find-none");
        let result = shown(find_name_in(test.dir(), "unicorn"));
        assert!(result.starts_with("I couldn't find anything called \"unicorn\""));
    }

    #[test]
    fn finds_text_with_line_numbers_in_a_folder() {
        let test = TestFolder::new("find-text");
        fs::create_dir(test.at("notes")).unwrap();
        fs::write(test.at("notes/todo.txt"), "buy milk\nsay Hello to Sam\n").unwrap();
        fs::write(test.at("notes/photo.jpg"), [0xFF, 0xD8, 0xFF]).unwrap();

        let result = shown(find_text("hello", &test.at("notes")));
        assert!(result.starts_with("Found \"hello\" once:"), "{result}");
        assert!(result.contains("todo.txt:2  say Hello to Sam"), "{result}");
    }

    #[test]
    fn finds_text_in_one_file_and_explains_files_that_are_not_text() {
        let test = TestFolder::new("find-text-file");
        fs::write(test.at("a.txt"), "one\ntwo\none again\n").unwrap();
        fs::write(test.at("photo.jpg"), [0xFF, 0xD8, 0xFF]).unwrap();

        assert!(shown(find_text("one", &test.at("a.txt"))).starts_with("Found \"one\" 2 times:"));
        assert!(shown(find_text("zebra", &test.at("a.txt"))).starts_with("I couldn't find"));
        let picture = find_text("one", &test.at("photo.jpg"));
        assert!(matches!(picture, Err(ActionError::NotText(_))));
    }

    #[test]
    fn says_how_many_files_it_could_not_search() {
        let test = TestFolder::new("find-skipped");
        fs::write(test.at("a.txt"), "hello").unwrap();
        fs::write(test.at("photo.jpg"), [0xFF, 0xD8, 0xFF]).unwrap();
        let result = shown(find_text("hello", &test.path()));
        assert!(result.contains("I skipped 1 file: not text"), "{result}");
    }

    #[test]
    fn cuts_very_long_lines_short() {
        let long = "x".repeat(300);
        assert_eq!(shorten(&long).chars().count(), MAX_LINE_LENGTH + 1);
    }
}
