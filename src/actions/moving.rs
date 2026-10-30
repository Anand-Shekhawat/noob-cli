//! Copying, moving, renaming and deleting (to the Trash).

use std::env;
use std::fs;
use std::path::Path;

use super::Reply;
use super::errors::{ActionError, friendly};
use super::paths::{check_free, existing, free_destination, is_inside, is_shortcut, same_thing};
use super::words::plural;

pub(super) fn copy(from: &str, to: &str) -> Result<Reply, ActionError> {
    let source = existing(from)?;
    let target = free_destination(source, to)?;
    // `symlink_metadata` describes a shortcut itself, not the thing it points to.
    let info = fs::symlink_metadata(source).map_err(|error| friendly(error, from))?;
    if info.file_type().is_symlink() {
        copy_shortcut(source, &target).map_err(|error| friendly(error, to))?;
        return Ok(Reply::Done(format!(
            "Copied the shortcut \"{from}\" to \"{}\"",
            target.display()
        )));
    }
    if info.is_file() {
        fs::copy(source, &target).map_err(|error| friendly(error, to))?;
        return Ok(Reply::Done(format!(
            "Copied \"{from}\" to \"{}\"",
            target.display()
        )));
    }
    if !info.is_dir() {
        return Err(ActionError::NotAFile(from.to_string()));
    }
    if is_inside(&target, source) {
        return Err(ActionError::IntoItself(from.to_string()));
    }
    let mut tally = Tally::default();
    copy_folder(source, &target, &mut tally).map_err(|error| {
        // Be honest: some files may already be in the copy.
        ActionError::Explained(format!(
            "I stopped partway, so \"{}\" only has some of the files ({}). The problem: {error}",
            target.display(),
            plural(tally.files, "file")
        ))
    })?;
    let skipped = match tally.skipped {
        0 => String::new(),
        count => format!(
            " I skipped {} that aren't normal files.",
            plural(count, "special item")
        ),
    };
    Ok(Reply::Done(format!(
        "Copied the folder \"{from}\" ({}) to \"{}\".{skipped}",
        plural(tally.files, "file"),
        target.display()
    )))
}

/// What a folder copy has done so far.
#[derive(Default)]
struct Tally {
    files: u64,
    skipped: u64,
}

/// Copies a folder and everything inside it. Calls itself for each folder inside:
/// that's called "recursion".
fn copy_folder(source: &Path, target: &Path, tally: &mut Tally) -> Result<(), ActionError> {
    let target_label = target.display().to_string();
    let source_label = source.display().to_string();
    fs::create_dir(target).map_err(|error| friendly(error, &target_label))?;

    let entries = fs::read_dir(source).map_err(|error| friendly(error, &source_label))?;
    for entry in entries {
        let entry = entry.map_err(|error| friendly(error, &source_label))?;
        let (from, to) = (entry.path(), target.join(entry.file_name()));
        let label = from.display().to_string();
        let kind = entry.file_type().map_err(|error| friendly(error, &label))?;
        if kind.is_dir() {
            copy_folder(&from, &to, tally)?;
        } else if kind.is_symlink() {
            copy_shortcut(&from, &to).map_err(|error| friendly(error, &label))?;
        } else if kind.is_file() {
            fs::copy(&from, &to).map_err(|error| friendly(error, &label))?;
            tally.files += 1;
        } else {
            // Pipes and devices aren't real files; copying one could wait forever.
            tally.skipped += 1;
        }
    }
    Ok(())
}

/// A shortcut is copied as a shortcut, never as the thing it points to.
#[cfg(unix)]
fn copy_shortcut(from: &Path, to: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(fs::read_link(from)?, to)
}

#[cfg(not(unix))]
fn copy_shortcut(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::copy(from, to).map(|_| ())
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
    // "rename notes.txt to Notes.txt" on a Mac: both names are the same file, so it's fine.
    if !same_thing(source, target) {
        check_free(target)?;
    }
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
    let path = Path::new(name);
    // ".", ".." and "/" have no file name of their own.
    if path.file_name().is_none() {
        return Err(ActionError::CantDelete(name.to_string()));
    }
    fs::symlink_metadata(path).map_err(|error| friendly(error, name))?;
    // Deleting a shortcut only removes the shortcut, never what it points to, so it's safe.
    if is_shortcut(path) {
        return Ok(());
    }
    // If a full path can't be worked out, there's nothing to compare, and the checks
    // above have already made sure the thing exists and isn't ".", ".." or "/".
    // Both sides go through `canonicalize`, so they're written the same way
    // (Windows can write one full path in two different styles).
    let here = env::current_dir().and_then(fs::canonicalize).ok();
    let home = env::home_dir().and_then(|home| fs::canonicalize(home).ok());
    if let Ok(full) = fs::canonicalize(path)
        && is_precious(&full, here.as_deref(), home.as_deref())
    {
        return Err(ActionError::CantDelete(name.to_string()));
    }
    Ok(())
}

/// True for the folder you're in (or any folder holding it), and your home folder
/// (or any folder holding that). Deleting those would pull the floor from under you.
fn is_precious(full: &Path, here: Option<&Path>, home: Option<&Path>) -> bool {
    let holds = |place: Option<&Path>| place.is_some_and(|place| place.starts_with(full));
    holds(here) || holds(home)
}

/// "Move the folder "photos" and the 12 things inside it to the Trash?"
pub(super) fn delete_question(name: &str) -> String {
    let is_folder = fs::symlink_metadata(name).is_ok_and(|info| info.is_dir());
    let what = match fs::read_dir(name) {
        Ok(items) if is_folder => match items.count() {
            0 => format!("the empty folder \"{name}\""),
            count => format!(
                "the folder \"{name}\" and the {} inside it",
                plural(count as u64, "thing")
            ),
        },
        _ => format!("\"{name}\""),
    };
    format!("Move {what} to the {}?", bin_name())
}

/// What this computer calls the place deleted things go.
fn bin_name() -> &'static str {
    if cfg!(windows) {
        "Recycle Bin"
    } else {
        "Trash"
    }
}

pub(super) fn delete(name: &str) -> Result<Reply, ActionError> {
    check_delete(name)?;
    send_to_trash(Path::new(name))?;
    Ok(Reply::Done(format!(
        "Moved \"{name}\" to the {}. (Changed your mind? You can get it back from there.)",
        bin_name()
    )))
}

#[cfg(not(test))]
fn send_to_trash(path: &Path) -> Result<(), ActionError> {
    trash_bin().delete(path).map_err(|error| {
        ActionError::Explained(format!(
            "I couldn't move it to the {}, so nothing was deleted. ({error})",
            bin_name()
        ))
    })
}

/// On a Mac, ask the system directly instead of asking the Finder app. It's faster
/// and doesn't pop up a "Terminal wants to control Finder" permission question.
#[cfg(all(not(test), target_os = "macos"))]
fn trash_bin() -> trash::TrashContext {
    use trash::macos::{DeleteMethod, TrashContextExtMacos};
    let mut bin = trash::TrashContext::default();
    bin.set_delete_method(DeleteMethod::NsFileManager);
    bin
}

#[cfg(all(not(test), not(target_os = "macos")))]
fn trash_bin() -> trash::TrashContext {
    trash::TrashContext::default()
}

/// Tests must not fill your real Trash with test files, so they use this stand-in.
/// (The real Trash is checked by hand, and the stand-in keeps every other step tested.)
#[cfg(test)]
fn send_to_trash(path: &Path) -> Result<(), ActionError> {
    let label = path.display().to_string();
    let info = fs::symlink_metadata(path).map_err(|error| friendly(error, &label))?;
    let result = if info.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    result.map_err(|error| friendly(error, &label))
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
    fn copies_whole_folders_with_everything_inside() {
        let test = TestFolder::new("copy-folder");
        fs::create_dir_all(test.at("photos/2024")).unwrap();
        fs::write(test.at("photos/cat.jpg"), "meow").unwrap();
        fs::write(test.at("photos/2024/dog.jpg"), "woof").unwrap();

        let reply = copy(&test.at("photos"), &test.at("backup")).unwrap();
        assert!(matches!(reply, Reply::Done(message) if message.contains("(2 files)")));
        assert_eq!(
            fs::read_to_string(test.at("backup/2024/dog.jpg")).unwrap(),
            "woof"
        );
        assert!(
            Path::new(&test.at("photos/cat.jpg")).exists(),
            "the original stays"
        );
    }

    #[test]
    fn will_not_copy_a_folder_into_itself() {
        let test = TestFolder::new("copy-into-itself");
        fs::create_dir(test.at("docs")).unwrap();
        let result = copy(&test.at("docs"), &test.at("docs"));
        assert!(matches!(result, Err(ActionError::IntoItself(_))));
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
    fn folders_holding_where_you_are_or_your_home_are_precious() {
        let here = Path::new("/Users/sam/projects/app");
        let home = Path::new("/Users/sam");
        assert!(is_precious(
            Path::new("/Users/sam/projects"),
            Some(here),
            Some(home)
        ));
        assert!(is_precious(Path::new("/Users"), Some(here), Some(home)));
        assert!(!is_precious(
            Path::new("/Users/sam/photos"),
            Some(here),
            Some(home)
        ));
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
    fn the_delete_question_says_how_much_is_inside_a_folder() {
        let test = TestFolder::new("delete-question");
        fs::create_dir(test.at("photos")).unwrap();
        fs::create_dir(test.at("empty")).unwrap();
        fs::write(test.at("photos/a.jpg"), "").unwrap();
        fs::write(test.at("photos/b.jpg"), "").unwrap();

        assert!(delete_question(&test.at("photos")).contains("and the 2 things inside it"));
        assert!(delete_question(&test.at("empty")).contains("the empty folder"));
    }

    #[test]
    fn deletes_files_and_full_folders() {
        let test = TestFolder::new("delete");
        fs::write(test.at("a.txt"), "").unwrap();
        fs::create_dir(test.at("photos")).unwrap();
        fs::write(test.at("photos/cat.jpg"), "").unwrap();

        delete(&test.at("a.txt")).unwrap();
        delete(&test.at("photos")).unwrap();

        assert!(!Path::new(&test.at("a.txt")).exists());
        assert!(!Path::new(&test.at("photos")).exists());
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_folder_copy_says_it_stopped_partway() {
        use std::os::unix::fs::PermissionsExt;
        let test = TestFolder::new("copy-partway");
        fs::create_dir(test.at("photos")).unwrap();
        fs::write(test.at("photos/locked.jpg"), "secret").unwrap();
        fs::set_permissions(
            test.at("photos/locked.jpg"),
            fs::Permissions::from_mode(0o000),
        )
        .unwrap();

        let result = copy(&test.at("photos"), &test.at("backup"));
        assert!(
            matches!(&result, Err(ActionError::Explained(message)) if message.contains("stopped partway")),
            "{result:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn folder_copies_skip_pipes_instead_of_getting_stuck() {
        let test = TestFolder::new("copy-pipe");
        fs::create_dir(test.at("box")).unwrap();
        fs::write(test.at("box/a.txt"), "hi").unwrap();
        crate::actions::test_support::make_pipe(&test, "box/pipe");

        let reply = copy(&test.at("box"), &test.at("copy")).unwrap();
        assert!(
            matches!(reply, Reply::Done(message) if message.contains("skipped 1 special item"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn copying_a_shortcut_copies_the_shortcut() {
        let test = TestFolder::new("copy-link");
        fs::write(test.at("real.txt"), "hi").unwrap();
        std::os::unix::fs::symlink(test.at("real.txt"), test.at("link")).unwrap();

        copy(&test.at("link"), &test.at("link2")).unwrap();
        assert!(
            fs::symlink_metadata(test.at("link2"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn renaming_only_the_capital_letters_works() {
        let test = TestFolder::new("rename-case");
        fs::write(test.at("notes.txt"), "hi").unwrap();
        rename(&test.at("notes.txt"), &test.at("Notes.txt")).unwrap();
        assert_eq!(fs::read_to_string(test.at("Notes.txt")).unwrap(), "hi");
    }

    #[cfg(unix)]
    #[test]
    fn a_shortcut_to_a_precious_folder_can_still_be_deleted() {
        let test = TestFolder::new("delete-link-home");
        let here = std::env::current_dir().unwrap();
        std::os::unix::fs::symlink(&here, test.at("link-to-here")).unwrap();
        assert!(check_delete(&test.at("link-to-here")).is_ok());
    }
}
