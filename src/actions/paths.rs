//! Small helpers about paths that several commands share.

use std::fs;
use std::path::{Path, PathBuf};

use super::errors::ActionError;

/// True if anything is at `path`, even a broken shortcut. (`Path::exists` follows
/// shortcuts, so it says "nothing here" for a broken one, which isn't safe.)
pub(super) fn taken(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Checks that `name` exists before we try to use it.
pub(super) fn existing(name: &str) -> Result<&Path, ActionError> {
    let path = Path::new(name);
    if taken(path) {
        Ok(path)
    } else {
        Err(ActionError::NotFound(name.to_string()))
    }
}

/// Where a copy or move should land. If `to` is a folder, the item goes inside it
/// with its own name.
pub(super) fn free_destination(source: &Path, to: &str) -> Result<PathBuf, ActionError> {
    let to = Path::new(to);
    let target = match source.file_name() {
        Some(file_name) if to.is_dir() => to.join(file_name),
        _ => to.to_path_buf(),
    };
    check_free(&target)?;
    Ok(target)
}

/// A destination must be empty (never overwrite!) and its folder must exist.
pub(super) fn check_free(target: &Path) -> Result<(), ActionError> {
    if taken(target) {
        return Err(ActionError::AlreadyExists(target.display().to_string()));
    }
    let folder = folder_of(target);
    if !folder.is_dir() {
        return Err(ActionError::NotFound(folder.display().to_string()));
    }
    Ok(())
}

/// The folder a path lives in. For a bare name like "a.txt", that's "." (here).
pub(super) fn folder_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// True if `target` would end up inside the folder `source` (like moving "docs" into "docs").
pub(super) fn is_inside(target: &Path, source: &Path) -> bool {
    // `canonicalize` turns paths like "./docs/../docs" into one full, standard form.
    match (
        fs::canonicalize(source),
        fs::canonicalize(folder_of(target)),
    ) {
        (Ok(source), Ok(destination)) => source.is_dir() && destination.starts_with(source),
        _ => false,
    }
}

/// Shows a folder the way people say it: your home folder is written as `~`.
pub fn short(path: &Path) -> String {
    short_with_home(path, std::env::home_dir().as_deref())
}

fn short_with_home(path: &Path, home: Option<&Path>) -> String {
    // `strip_prefix` compares whole folder names, so "/Users/samantha" doesn't match "/Users/sam".
    match home.and_then(|home| path.strip_prefix(home).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~/{}", rest.display()),
        None => path.display().to_string(),
    }
}

/// True if `path` is a shortcut (symlink), even a broken one.
pub(super) fn is_shortcut(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|info| info.file_type().is_symlink())
}

/// True if two names point at the very same file. On a Mac, "a.txt" and "A.TXT"
/// usually do, because Mac disks ignore capital letters in names.
#[cfg(unix)]
pub(super) fn same_thing(a: &Path, b: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (fs::symlink_metadata(a), fs::symlink_metadata(b)) {
        // A file's "inode" number on its "device" is its true identity, whatever its name.
        (Ok(a), Ok(b)) => a.ino() == b.ino() && a.dev() == b.dev(),
        _ => false,
    }
}

#[cfg(not(unix))]
pub(super) fn same_thing(_a: &Path, _b: &Path) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_the_home_folder_as_a_squiggle() {
        let home = Path::new("/Users/sam");
        assert_eq!(
            short_with_home(Path::new("/Users/sam/photos"), Some(home)),
            "~/photos"
        );
        assert_eq!(short_with_home(home, Some(home)), "~");
        assert_eq!(short_with_home(Path::new("/tmp"), Some(home)), "/tmp");
        assert_eq!(
            short_with_home(Path::new("/Users/samantha"), Some(home)),
            "/Users/samantha"
        );
    }

    #[test]
    fn a_bare_name_lives_in_the_current_folder() {
        assert_eq!(folder_of(Path::new("a.txt")), Path::new("."));
        assert_eq!(folder_of(Path::new("docs/a.txt")), Path::new("docs"));
    }
}
