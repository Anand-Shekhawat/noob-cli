//! Looking at what's in a folder.

use std::fs;
use std::path::Path;

use super::Reply;
use super::errors::{ActionError, friendly};
use crate::style;

pub(super) fn show_files(folder: &Path) -> Result<Reply, ActionError> {
    let label = folder.display().to_string();
    let entries = fs::read_dir(folder)
        .and_then(|listing| listing.collect::<Result<Vec<_>, _>>())
        .map_err(|error| friendly(error, &label))?;

    // (is_file, name): sorting puts folders (false) before files (true).
    let mut items: Vec<(bool, String)> = entries
        .iter()
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            (!entry.path().is_dir(), name)
        })
        .filter(|(_, name)| !name.starts_with('.')) // names starting with "." are hidden
        .collect();
    items.sort_by_key(|(is_file, name)| (*is_file, name.to_lowercase()));

    if items.is_empty() {
        return Ok(Reply::Show("This folder is empty.".to_string()));
    }
    let lines: Vec<String> = items
        .iter()
        .map(|(is_file, name)| {
            if *is_file {
                format!("  📄 {name}")
            } else {
                format!("  📁 {}", style::folder(&format!("{name}/")))
            }
        })
        .collect();
    Ok(Reply::Show(lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::test_support::TestFolder;

    #[test]
    fn lists_folders_first_and_hides_hidden_files() {
        let test = TestFolder::new("show");
        fs::write(test.at("b.txt"), "").unwrap();
        fs::write(test.at(".secret"), "").unwrap();
        fs::create_dir(test.at("zebra")).unwrap();

        let listing = show_files(test.dir()).unwrap();
        assert_eq!(listing, Reply::Show("  📁 zebra/\n  📄 b.txt".to_string()));
    }

    #[test]
    fn says_when_a_folder_is_empty() {
        let test = TestFolder::new("show-empty");
        let listing = show_files(test.dir()).unwrap();
        assert_eq!(listing, Reply::Show("This folder is empty.".to_string()));
    }
}
