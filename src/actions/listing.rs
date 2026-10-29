//! Looking at what's in a folder: show files, show all files, show details,
//! show tree and show size.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::Reply;
use super::errors::{ActionError, friendly};
use super::paths::existing;
use super::walk::{self, Next};
use super::words::{human_age, human_size, plural};
use crate::style;

/// How many folders deep `show tree` goes.
const TREE_DEPTH: usize = 3;
/// `show tree` stops after this many lines, so a huge folder can't flood the screen.
const TREE_LINES: usize = 200;
/// Longer names don't push the `show details` columns any further right.
const NAME_COLUMN_MAX: usize = 40;

/// One thing inside a folder.
struct Entry {
    name: String,
    path: PathBuf,
    is_folder: bool,
}

/// What's inside `folder`: folders first, then A to Z.
fn sorted_entries(folder: &Path, include_hidden: bool) -> Result<Vec<Entry>, ActionError> {
    let label = folder.display().to_string();
    let listing = fs::read_dir(folder)
        .and_then(|listing| listing.collect::<Result<Vec<_>, _>>())
        .map_err(|error| friendly(error, &label))?;

    let mut entries: Vec<Entry> = listing
        .iter()
        .map(|item| {
            let path = item.path();
            Entry {
                name: item.file_name().to_string_lossy().into_owned(),
                is_folder: path.is_dir(),
                path,
            }
        })
        // Names starting with "." are hidden (usually settings files).
        .filter(|entry| include_hidden || !entry.name.starts_with('.'))
        .collect();
    entries.sort_by_key(|entry| (!entry.is_folder, entry.name.to_lowercase()));
    Ok(entries)
}

fn label(entry: &Entry) -> String {
    if entry.is_folder {
        format!("📁 {}", style::folder(&format!("{}/", entry.name)))
    } else {
        format!("📄 {}", entry.name)
    }
}

fn empty_folder(include_hidden: bool) -> Reply {
    let message = if include_hidden {
        "This folder is empty."
    } else {
        "This folder is empty. (To include hidden files, try: show all files)"
    };
    Reply::Show(message.to_string())
}

pub(super) fn show_files(folder: &Path, include_hidden: bool) -> Result<Reply, ActionError> {
    let entries = sorted_entries(folder, include_hidden)?;
    if entries.is_empty() {
        return Ok(empty_folder(include_hidden));
    }
    let lines: Vec<String> = entries
        .iter()
        .map(|entry| format!("  {}", label(entry)))
        .collect();
    Ok(Reply::Show(lines.join("\n")))
}

pub(super) fn show_details(folder: &Path) -> Result<Reply, ActionError> {
    let entries = sorted_entries(folder, false)?;
    if entries.is_empty() {
        return Ok(empty_folder(false));
    }
    let width = entries
        .iter()
        .map(|entry| entry.name.chars().count() + 1)
        .max()
        .unwrap_or(0)
        .min(NAME_COLUMN_MAX);
    let now = SystemTime::now();
    let lines: Vec<String> = entries
        .iter()
        .map(|entry| detail_line(entry, width, now))
        .collect();
    Ok(Reply::Show(lines.join("\n")))
}

fn detail_line(entry: &Entry, width: usize, now: SystemTime) -> String {
    // Pad the name before colouring it: colour codes would confuse the spacing.
    let (icon, name) = if entry.is_folder {
        (
            "📁",
            style::folder(&format!("{:<width$}", format!("{}/", entry.name))),
        )
    } else {
        ("📄", format!("{:<width$}", entry.name))
    };
    let metadata = fs::metadata(&entry.path).ok();
    let size = if entry.is_folder {
        fs::read_dir(&entry.path).map_or("?".to_string(), |items| {
            plural(items.count() as u64, "item")
        })
    } else {
        metadata
            .as_ref()
            .map_or("?".to_string(), |info| human_size(info.len()))
    };
    let changed = metadata
        .and_then(|info| info.modified().ok())
        .and_then(|time| now.duration_since(time).ok())
        .map(|age| format!("changed {}", human_age(age)))
        .unwrap_or_default();
    format!("  {icon} {name}  {size:>10}  {changed}")
}

pub(super) fn show_tree(folder: &Path) -> Result<Reply, ActionError> {
    let root_name = fs::canonicalize(folder)
        .ok()
        .and_then(|full| {
            full.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| folder.display().to_string());
    let mut lines = vec![style::folder(&format!("{root_name}/"))];
    let complete = add_branches(folder, "", 1, &mut lines)?;
    if !complete {
        lines.push(format!(
            "… (I stopped after {TREE_LINES} lines. Try show tree on a smaller folder.)"
        ));
    }
    Ok(Reply::Show(lines.join("\n")))
}

/// Adds one folder's contents to the tree drawing. Returns `false` if the tree got too long.
fn add_branches(
    folder: &Path,
    prefix: &str,
    depth: usize,
    lines: &mut Vec<String>,
) -> Result<bool, ActionError> {
    let entries = match sorted_entries(folder, false) {
        Ok(entries) => entries,
        Err(error) if depth == 1 => return Err(error),
        Err(_) => {
            lines.push(format!("{prefix}└── (I can't look inside)"));
            return Ok(true);
        }
    };
    for (index, entry) in entries.iter().enumerate() {
        if lines.len() > TREE_LINES {
            return Ok(false);
        }
        let last = index + 1 == entries.len();
        let branch = if last { "└── " } else { "├── " };
        lines.push(format!("{prefix}{branch}{}", tree_label(entry)));

        // Never follow shortcuts: one pointing back up would go round in circles.
        if !entry.is_folder || entry.path.is_symlink() {
            continue;
        }
        let child_prefix = format!("{prefix}{}", if last { "    " } else { "│   " });
        if depth < TREE_DEPTH {
            if !add_branches(&entry.path, &child_prefix, depth + 1, lines)? {
                return Ok(false);
            }
        } else if fs::read_dir(&entry.path).is_ok_and(|mut items| items.next().is_some()) {
            lines.push(format!("{child_prefix}└── …"));
        }
    }
    Ok(true)
}

fn tree_label(entry: &Entry) -> String {
    if entry.is_folder {
        style::folder(&format!("{}/", entry.name))
    } else {
        entry.name.clone()
    }
}

pub(super) fn show_size(name: &str) -> Result<Reply, ActionError> {
    let path = existing(name)?;
    let info = fs::symlink_metadata(path).map_err(|error| friendly(error, name))?;
    if !info.is_dir() {
        return Ok(Reply::Show(format!(
            "\"{name}\" is {}.",
            human_size(info.len())
        )));
    }
    let (mut files, mut bytes) = (0, 0);
    let summary = walk::walk(path, true, |_, metadata| {
        if metadata.is_file() {
            files += 1;
            bytes += metadata.len();
        }
        Next::Continue
    });
    Ok(Reply::Show(format!(
        "\"{name}\" holds {}, {} in total.{}",
        plural(files, "file"),
        human_size(bytes),
        summary.note()
    )))
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
    fn lists_folders_first_and_hides_hidden_files() {
        let test = TestFolder::new("show");
        fs::write(test.at("b.txt"), "").unwrap();
        fs::write(test.at(".secret"), "").unwrap();
        fs::create_dir(test.at("zebra")).unwrap();

        assert_eq!(
            shown(show_files(test.dir(), false)),
            "  📁 zebra/\n  📄 b.txt"
        );
        assert!(shown(show_files(test.dir(), true)).contains(".secret"));
    }

    #[test]
    fn says_when_a_folder_is_empty_and_mentions_hidden_files() {
        let test = TestFolder::new("show-empty");
        assert!(shown(show_files(test.dir(), false)).starts_with("This folder is empty."));
        assert_eq!(shown(show_files(test.dir(), true)), "This folder is empty.");
    }

    #[test]
    fn showing_a_file_as_a_folder_is_explained() {
        let test = TestFolder::new("show-file");
        fs::write(test.at("a.txt"), "").unwrap();
        let result = show_files(Path::new(&test.at("a.txt")), false);
        assert!(matches!(result, Err(ActionError::NotAFolder(_))));
    }

    #[test]
    fn details_show_sizes_and_item_counts() {
        let test = TestFolder::new("details");
        fs::write(test.at("notes.txt"), "hello").unwrap();
        fs::create_dir(test.at("docs")).unwrap();
        fs::write(test.at("docs/a.txt"), "").unwrap();

        let details = shown(show_details(test.dir()));
        assert!(details.contains("1 item"), "{details}");
        assert!(details.contains("5 bytes"), "{details}");
        assert!(details.contains("changed just now"), "{details}");
    }

    #[test]
    fn draws_a_tree_of_folders_inside_folders() {
        let test = TestFolder::new("tree");
        fs::create_dir_all(test.at("a/b")).unwrap();
        fs::write(test.at("a/b/deep.txt"), "").unwrap();
        fs::write(test.at("top.txt"), "").unwrap();

        let tree = shown(show_tree(test.dir()));
        let expected = "├── a/\n│   └── b/\n│       └── deep.txt\n└── top.txt";
        assert!(tree.ends_with(expected), "{tree}");
    }

    #[test]
    fn measures_files_and_whole_folders() {
        let test = TestFolder::new("size");
        fs::create_dir(test.at("docs")).unwrap();
        fs::write(test.at("docs/a.txt"), "12345").unwrap();
        fs::write(test.at("docs/.hidden"), "123").unwrap();

        assert_eq!(
            shown(show_size(&test.at("docs/a.txt"))),
            format!("\"{}\" is 5 bytes.", test.at("docs/a.txt"))
        );
        let folder = shown(show_size(&test.at("docs")));
        assert!(
            folder.contains("holds 2 files, 8 bytes in total"),
            "hidden files count too: {folder}"
        );
    }
}
