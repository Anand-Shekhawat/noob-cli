//! Copying, moving, renaming and deleting.

use std::fs;
use std::path::Path;

use super::Reply;
use super::errors::{ActionError, friendly};
use super::paths::{check_free, existing, free_destination, is_inside};

pub(super) fn copy(from: &str, to: &str) -> Result<Reply, ActionError> {
    let source = existing(from)?;
    if source.is_dir() {
        return Err(ActionError::IsAFolder(from.to_string()));
    }
    let target = free_destination(source, to)?;
    fs::copy(source, &target).map_err(|error| friendly(error, to))?;
    Ok(Reply::Done(format!(
        "Copied \"{from}\" to \"{}\"",
        target.display()
    )))
}

pub(super) fn move_into(from: &str, to: &str) -> Result<Reply, ActionError> {
    let source = existing(from)?;
    let target = free_destination(source, to)?;
    relocate(source, &target)?;
    Ok(Reply::Done(format!(
        "Moved \"{from}\" to \"{}\"",
        target.display()
    )))
}

pub(super) fn rename(from: &str, to: &str) -> Result<Reply, ActionError> {
    let source = existing(from)?;
    let target = Path::new(to);
    check_free(target)?;
    relocate(source, target)?;
    Ok(Reply::Done(format!("Renamed \"{from}\" to \"{to}\"")))
}

/// The shared last step of move and rename.
fn relocate(source: &Path, target: &Path) -> Result<(), ActionError> {
    if is_inside(target, source) {
        return Err(ActionError::IntoItself(source.display().to_string()));
    }
    // We already know `source` exists, so any error is about the destination.
    fs::rename(source, target).map_err(|error| friendly(error, &target.display().to_string()))
}

/// Checks a delete can happen before we ask "are you sure?".
pub(super) fn check_delete(name: &str) -> Result<(), ActionError> {
    // ".", ".." and "/" have no file name of their own.
    if Path::new(name).file_name().is_none() {
        return Err(ActionError::CantDelete(name.to_string()));
    }
    fs::symlink_metadata(name)
        .map(|_| ())
        .map_err(|error| friendly(error, name))
}

pub(super) fn delete(name: &str) -> Result<Reply, ActionError> {
    check_delete(name)?;
    // `symlink_metadata` looks at a shortcut (link) itself instead of following it.
    let info = fs::symlink_metadata(name).map_err(|error| friendly(error, name))?;
    if info.is_dir() {
        // `remove_dir` only works on empty folders, which is exactly the safety we want.
        fs::remove_dir(name).map_err(|error| friendly(error, name))?;
        Ok(Reply::Done(format!("Deleted folder \"{name}\"")))
    } else {
        fs::remove_file(name).map_err(|error| friendly(error, name))?;
        Ok(Reply::Done(format!("Deleted \"{name}\"")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::test_support::TestFolder;

    #[test]
    fn copies_to_a_new_name_or_into_a_folder() {
        let test = TestFolder::new("copy");
        fs::write(test.at("a.txt"), "hi").unwrap();
        fs::create_dir(test.at("docs")).unwrap();

        copy(&test.at("a.txt"), &test.at("b.txt")).unwrap();
        copy(&test.at("a.txt"), &test.at("docs")).unwrap();

        assert_eq!(fs::read_to_string(test.at("b.txt")).unwrap(), "hi");
        assert_eq!(fs::read_to_string(test.at("docs/a.txt")).unwrap(), "hi");
        assert!(Path::new(&test.at("a.txt")).exists(), "the original stays");
    }

    #[test]
    fn copy_never_overwrites() {
        let test = TestFolder::new("copy-overwrite");
        fs::write(test.at("a.txt"), "new").unwrap();
        fs::write(test.at("b.txt"), "old").unwrap();

        let result = copy(&test.at("a.txt"), &test.at("b.txt"));
        assert!(matches!(result, Err(ActionError::AlreadyExists(_))));
        assert_eq!(fs::read_to_string(test.at("b.txt")).unwrap(), "old");
    }

    #[test]
    fn moves_into_a_folder() {
        let test = TestFolder::new("move");
        fs::write(test.at("a.txt"), "hi").unwrap();
        fs::create_dir(test.at("docs")).unwrap();

        move_into(&test.at("a.txt"), &test.at("docs")).unwrap();

        assert!(!Path::new(&test.at("a.txt")).exists());
        assert!(Path::new(&test.at("docs/a.txt")).exists());
    }

    #[test]
    fn renames_but_never_overwrites() {
        let test = TestFolder::new("rename");
        fs::write(test.at("old.txt"), "hi").unwrap();
        fs::write(test.at("taken.txt"), "keep").unwrap();

        rename(&test.at("old.txt"), &test.at("new.txt")).unwrap();
        assert!(Path::new(&test.at("new.txt")).exists());

        let clash = rename(&test.at("new.txt"), &test.at("taken.txt"));
        assert!(matches!(clash, Err(ActionError::AlreadyExists(_))));
        assert_eq!(fs::read_to_string(test.at("taken.txt")).unwrap(), "keep");
    }

    #[test]
    fn moving_something_missing_is_explained() {
        let test = TestFolder::new("move-missing");
        let result = move_into(&test.at("ghost.txt"), &test.at("docs"));
        assert!(matches!(result, Err(ActionError::NotFound(_))));
    }

    #[cfg(unix)]
    #[test]
    fn never_writes_through_a_broken_shortcut() {
        let test = TestFolder::new("dangling");
        fs::write(test.at("a.txt"), "hi").unwrap();
        // A "symlink" is a shortcut. This one points at a file that doesn't exist.
        std::os::unix::fs::symlink(test.at("nowhere.txt"), test.at("link")).unwrap();

        let copied = copy(&test.at("a.txt"), &test.at("link"));
        assert!(matches!(copied, Err(ActionError::AlreadyExists(_))));
        assert!(!Path::new(&test.at("nowhere.txt")).exists());

        let renamed = rename(&test.at("a.txt"), &test.at("link"));
        assert!(matches!(renamed, Err(ActionError::AlreadyExists(_))));
    }

    #[test]
    fn blames_the_missing_folder_not_the_file() {
        let test = TestFolder::new("missing-folder");
        fs::write(test.at("a.txt"), "hi").unwrap();
        let missing = test.at("nope");

        let renamed = rename(&test.at("a.txt"), &test.at("nope/b.txt"));
        assert!(matches!(renamed, Err(ActionError::NotFound(name)) if name == missing));

        let moved = move_into(&test.at("a.txt"), &test.at("nope/b.txt"));
        assert!(matches!(moved, Err(ActionError::NotFound(name)) if name == missing));
    }

    #[test]
    fn will_not_move_a_folder_inside_itself() {
        let test = TestFolder::new("into-itself");
        fs::create_dir(test.at("docs")).unwrap();

        let result = move_into(&test.at("docs"), &test.at("docs"));
        assert!(matches!(result, Err(ActionError::IntoItself(_))));
        assert!(Path::new(&test.at("docs")).is_dir());
    }

    #[test]
    fn will_not_delete_the_current_folder_or_the_whole_disk() {
        for name in [".", "..", "/"] {
            let result = check_delete(name);
            assert!(matches!(result, Err(ActionError::CantDelete(_))), "{name}");
        }
    }

    #[test]
    fn checks_the_file_exists_before_asking_to_delete_it() {
        let test = TestFolder::new("ready");
        fs::write(test.at("real.txt"), "").unwrap();

        let ghost = check_delete(&test.at("ghost.txt"));
        assert!(matches!(ghost, Err(ActionError::NotFound(_))));
        assert!(check_delete(&test.at("real.txt")).is_ok());
    }

    #[test]
    fn deletes_files_and_empty_folders() {
        let test = TestFolder::new("delete");
        fs::write(test.at("a.txt"), "").unwrap();
        fs::create_dir(test.at("empty")).unwrap();

        delete(&test.at("a.txt")).unwrap();
        delete(&test.at("empty")).unwrap();

        assert!(!Path::new(&test.at("a.txt")).exists());
        assert!(!Path::new(&test.at("empty")).exists());
    }

    #[test]
    fn refuses_to_delete_a_folder_with_things_inside() {
        let test = TestFolder::new("delete-full");
        fs::create_dir(test.at("photos")).unwrap();
        fs::write(test.at("photos/cat.jpg"), "").unwrap();

        let result = delete(&test.at("photos"));
        assert!(matches!(result, Err(ActionError::FolderNotEmpty(_))));
        assert!(Path::new(&test.at("photos/cat.jpg")).exists());
    }
}
