//! Visits everything inside a folder, and inside the folders inside it, and so on.
//! Used by `show size`, `find file` and `find "text" in`.

use std::fs;
use std::path::Path;

/// What the visitor wants next.
pub(super) enum Next {
    Continue,
    Stop,
}

/// What happened during a walk, so we can be honest about anything we missed.
#[derive(Default)]
pub(super) struct Summary {
    /// Folders or items we weren't allowed to look at.
    pub unreadable: usize,
    /// The visitor asked to stop early (usually: "that's enough results").
    pub stopped: bool,
}

impl Summary {
    /// A short sentence to add to a result, or nothing if we saw everything.
    pub fn note(&self) -> String {
        match self.unreadable {
            0 => String::new(),
            1 => "\n(I couldn't look inside 1 place.)".to_string(),
            count => format!("\n(I couldn't look inside {count} places.)"),
        }
    }
}

/// Calls `visit` for every file and folder inside `root`.
/// Shortcuts (symlinks) are visited but never followed, so a shortcut
/// pointing back up can't send us round in circles.
pub(super) fn walk(
    root: &Path,
    include_hidden: bool,
    mut visit: impl FnMut(&Path, &fs::Metadata) -> Next,
) -> Summary {
    let mut summary = Summary::default();
    // A "to do" pile of folders still to look inside.
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        let Ok(entries) = fs::read_dir(&folder) else {
            summary.unreadable += 1;
            continue;
        };
        for entry in entries {
            let Ok(entry) = entry else {
                summary.unreadable += 1;
                continue;
            };
            if !include_hidden && entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let path = entry.path();
            let Ok(metadata) = fs::symlink_metadata(&path) else {
                summary.unreadable += 1;
                continue;
            };
            if let Next::Stop = visit(&path, &metadata) {
                summary.stopped = true;
                return summary;
            }
            if metadata.is_dir() {
                folders.push(path);
            }
        }
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::test_support::TestFolder;

    #[test]
    fn visits_everything_inside_but_skips_hidden_things_when_asked() {
        let test = TestFolder::new("walk");
        fs::create_dir_all(test.at("a/b")).unwrap();
        fs::write(test.at("a/b/deep.txt"), "").unwrap();
        fs::write(test.at(".hidden"), "").unwrap();

        let mut seen = 0;
        walk(test.dir(), false, |_, _| {
            seen += 1;
            Next::Continue
        });
        assert_eq!(seen, 3, "a, a/b and a/b/deep.txt");

        let mut with_hidden = 0;
        walk(test.dir(), true, |_, _| {
            with_hidden += 1;
            Next::Continue
        });
        assert_eq!(with_hidden, 4);
    }

    #[test]
    fn stops_when_asked() {
        let test = TestFolder::new("walk-stop");
        for name in ["a", "b", "c"] {
            fs::write(test.at(name), "").unwrap();
        }
        let summary = walk(test.dir(), false, |_, _| Next::Stop);
        assert!(summary.stopped);
    }
}
