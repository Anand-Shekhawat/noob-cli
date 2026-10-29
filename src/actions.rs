//! Does the real work for each command, using Rust's standard library (`std::fs`).

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::command::Command;
use crate::help;

/// The special characters that tell a terminal to wipe the screen.
const CLEAR_SCREEN: &str = "\x1B[2J\x1B[1;1H";

/// Everything that can go wrong, in words a beginner understands.
#[derive(Debug)]
pub enum ActionError {
    NotFound(String),
    AlreadyExists(String),
    FolderNotEmpty(String),
    IsAFolder(String),
    NotText(String),
    NotAllowed(String),
    IntoItself(String),
    CantDelete(String),
    Other(String),
}

// This lets `ActionError` work with Rust's standard error tools, like `?` into `Box<dyn Error>`.
impl std::error::Error for ActionError {}

// `Display` decides how an error looks when printed with `{}`.
impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::NotFound(name) => {
                write!(
                    f,
                    "I couldn't find \"{name}\". Type `show files` to see what's here."
                )
            }
            ActionError::AlreadyExists(name) => {
                write!(
                    f,
                    "\"{name}\" already exists. Pick another name, or delete it first."
                )
            }
            ActionError::FolderNotEmpty(name) => write!(
                f,
                "\"{name}\" is not empty (it may hold hidden files). \
                 For safety, Noob only deletes empty folders."
            ),
            ActionError::IsAFolder(name) => {
                write!(
                    f,
                    "\"{name}\" is a folder. This only works with files for now."
                )
            }
            ActionError::NotText(name) => {
                write!(f, "\"{name}\" isn't a text file, so I can't show it.")
            }
            ActionError::NotAllowed(name) => {
                write!(
                    f,
                    "Your computer won't let me touch \"{name}\" (permission denied)."
                )
            }
            ActionError::IntoItself(name) => {
                write!(f, "I can't put \"{name}\" inside itself.")
            }
            ActionError::CantDelete(name) => write!(
                f,
                "Noob won't delete \"{name}\": that's the folder you're in, \
                 one above it, or the whole disk."
            ),
            ActionError::Other(details) => write!(f, "Something went wrong: {details}"),
        }
    }
}

/// Turns a low-level `io::Error` into one of our friendly errors.
fn friendly(error: io::Error, name: &str) -> ActionError {
    let name = name.to_string();
    match error.kind() {
        io::ErrorKind::NotFound => ActionError::NotFound(name),
        io::ErrorKind::AlreadyExists => ActionError::AlreadyExists(name),
        io::ErrorKind::PermissionDenied => ActionError::NotAllowed(name),
        io::ErrorKind::InvalidData => ActionError::NotText(name),
        io::ErrorKind::DirectoryNotEmpty => ActionError::FolderNotEmpty(name),
        io::ErrorKind::CrossesDevices => ActionError::Other(format!(
            "\"{name}\" is on a different drive, and Noob can't move between drives yet."
        )),
        _ => ActionError::Other(error.to_string()),
    }
}

/// Checks that a risky command can actually run, so we never ask
/// "are you sure?" about something that is going to fail anyway.
pub fn check_ready(command: &Command) -> Result<(), ActionError> {
    match command {
        Command::Delete(name) => {
            // ".", ".." and "/" have no file name of their own.
            if Path::new(name).file_name().is_none() {
                return Err(ActionError::CantDelete(name.to_string()));
            }
            fs::symlink_metadata(name)
                .map(|_| ())
                .map_err(|error| friendly(error, name))
        }
        _ => Ok(()),
    }
}

/// Runs one command and returns the message to show the user.
pub fn execute(command: &Command) -> Result<String, ActionError> {
    match command {
        Command::WhereAmI => where_am_i(),
        Command::ShowFiles => show_files(Path::new(".")),
        Command::MakeFolder(name) => make_folder(name),
        Command::MakeFile(name) => make_file(name),
        Command::Read(name) => read(name),
        Command::Copy { from, to } => copy(from, to),
        Command::Move { from, to } => move_into(from, to),
        Command::Rename { from, to } => rename(from, to),
        Command::Delete(name) => delete(name),
        Command::Clear => Ok(CLEAR_SCREEN.to_string()),
        Command::Help(topic) => Ok(help::text(topic.as_deref())),
        Command::Bye => Ok("Bye! See you next time.".to_string()),
    }
}

fn where_am_i() -> Result<String, ActionError> {
    let here = std::env::current_dir().map_err(|error| friendly(error, "."))?;
    Ok(format!("You are in {}", here.display()))
}

fn show_files(folder: &Path) -> Result<String, ActionError> {
    let label = folder.display().to_string();
    let entries = fs::read_dir(folder)
        .and_then(|listing| listing.collect::<Result<Vec<_>, _>>())
        .map_err(|error| friendly(error, &label))?;

    // (is_file, name): sorting puts folders (false) before files (true).
    let mut items: Vec<(bool, String)> = entries
        .iter()
        .map(|entry| {
            (
                !entry.path().is_dir(),
                entry.file_name().to_string_lossy().into_owned(),
            )
        })
        .filter(|(_, name)| !name.starts_with('.')) // names starting with "." are hidden
        .collect();
    items.sort_by_key(|(is_file, name)| (*is_file, name.to_lowercase()));

    if items.is_empty() {
        return Ok("This folder is empty.".to_string());
    }
    let lines: Vec<String> = items
        .iter()
        .map(|(is_file, name)| {
            if *is_file {
                format!("  📄 {name}")
            } else {
                format!("  📁 {name}/")
            }
        })
        .collect();
    Ok(lines.join("\n"))
}

fn make_folder(name: &str) -> Result<String, ActionError> {
    let path = Path::new(name);
    if taken(path) {
        return Err(ActionError::AlreadyExists(name.to_string()));
    }
    fs::create_dir_all(path).map_err(|error| friendly(error, name))?;
    Ok(format!("✓ Made folder \"{name}\""))
}

fn make_file(name: &str) -> Result<String, ActionError> {
    // `create_new` refuses to touch a file that already exists, so nothing gets wiped.
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(name)
        .map_err(|error| friendly(error, name))?;
    Ok(format!("✓ Made file \"{name}\""))
}

fn read(name: &str) -> Result<String, ActionError> {
    if Path::new(name).is_dir() {
        return Err(ActionError::IsAFolder(name.to_string()));
    }
    let text = fs::read_to_string(name).map_err(|error| friendly(error, name))?;
    if text.trim().is_empty() {
        return Ok(format!("\"{name}\" is empty."));
    }
    Ok(text.trim_end().to_string())
}

fn copy(from: &str, to: &str) -> Result<String, ActionError> {
    let source = existing(from)?;
    if source.is_dir() {
        return Err(ActionError::IsAFolder(from.to_string()));
    }
    let target = free_destination(source, to)?;
    fs::copy(source, &target).map_err(|error| friendly(error, to))?;
    Ok(format!("✓ Copied \"{from}\" to \"{}\"", target.display()))
}

fn move_into(from: &str, to: &str) -> Result<String, ActionError> {
    let source = existing(from)?;
    let target = free_destination(source, to)?;
    relocate(source, &target)?;
    Ok(format!("✓ Moved \"{from}\" to \"{}\"", target.display()))
}

fn rename(from: &str, to: &str) -> Result<String, ActionError> {
    let source = existing(from)?;
    let target = Path::new(to);
    check_free(target)?;
    relocate(source, target)?;
    Ok(format!("✓ Renamed \"{from}\" to \"{to}\""))
}

/// The shared last step of move and rename.
fn relocate(source: &Path, target: &Path) -> Result<(), ActionError> {
    if is_inside(target, source) {
        return Err(ActionError::IntoItself(source.display().to_string()));
    }
    // We already know `source` exists, so any error is about the destination.
    fs::rename(source, target).map_err(|error| friendly(error, &target.display().to_string()))
}

fn delete(name: &str) -> Result<String, ActionError> {
    check_ready(&Command::Delete(name.to_string()))?;
    // `symlink_metadata` looks at a shortcut (link) itself instead of following it.
    let info = fs::symlink_metadata(name).map_err(|error| friendly(error, name))?;
    if info.is_dir() {
        // `remove_dir` only works on empty folders, which is exactly the safety we want.
        fs::remove_dir(name).map_err(|error| friendly(error, name))?;
        Ok(format!("✓ Deleted folder \"{name}\""))
    } else {
        fs::remove_file(name).map_err(|error| friendly(error, name))?;
        Ok(format!("✓ Deleted \"{name}\""))
    }
}

/// True if anything is at `path`, even a broken shortcut. (`Path::exists` follows
/// shortcuts, so it says "nothing here" for a broken one, which isn't safe.)
fn taken(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

/// Checks that `name` exists before we try to copy or move it.
fn existing(name: &str) -> Result<&Path, ActionError> {
    let path = Path::new(name);
    if taken(path) {
        Ok(path)
    } else {
        Err(ActionError::NotFound(name.to_string()))
    }
}

/// Where a copy or move should land. If `to` is a folder, the item goes inside it
/// with its own name.
fn free_destination(source: &Path, to: &str) -> Result<PathBuf, ActionError> {
    let to = Path::new(to);
    let target = match source.file_name() {
        Some(file_name) if to.is_dir() => to.join(file_name),
        _ => to.to_path_buf(),
    };
    check_free(&target)?;
    Ok(target)
}

/// A destination must be empty (never overwrite!) and its folder must exist.
fn check_free(target: &Path) -> Result<(), ActionError> {
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
fn folder_of(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// True if `target` would end up inside the folder `source` (like moving "docs" into "docs").
fn is_inside(target: &Path, source: &Path) -> bool {
    // `canonicalize` turns paths like "./docs/../docs" into one full, standard form.
    match (
        fs::canonicalize(source),
        fs::canonicalize(folder_of(target)),
    ) {
        (Ok(source), Ok(destination)) => source.is_dir() && destination.starts_with(source),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh, empty folder for one test. It deletes itself when the test ends,
    /// because Rust runs `drop` automatically when a value goes out of scope.
    struct TestFolder(PathBuf);

    impl TestFolder {
        fn new(test_name: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("noob-test-{test_name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&path); // clean up leftovers from a crashed run
            fs::create_dir_all(&path).expect("create test folder");
            TestFolder(path)
        }

        /// The full path of `name` inside this test folder, as text.
        fn at(&self, name: &str) -> String {
            self.0.join(name).display().to_string()
        }
    }

    impl Drop for TestFolder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
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
            let result = check_ready(&Command::Delete(name.to_string()));
            assert!(matches!(result, Err(ActionError::CantDelete(_))), "{name}");
        }
    }

    #[test]
    fn checks_the_file_exists_before_asking_to_delete_it() {
        let test = TestFolder::new("ready");
        fs::write(test.at("real.txt"), "").unwrap();

        let ghost = check_ready(&Command::Delete(test.at("ghost.txt")));
        assert!(matches!(ghost, Err(ActionError::NotFound(_))));
        assert!(check_ready(&Command::Delete(test.at("real.txt"))).is_ok());
    }

    #[test]
    fn makes_nested_folders() {
        let test = TestFolder::new("nested");
        make_folder(&test.at("a/b/c")).unwrap();
        assert!(Path::new(&test.at("a/b/c")).is_dir());
    }

    #[test]
    fn will_not_make_a_folder_twice() {
        let test = TestFolder::new("folder-twice");
        make_folder(&test.at("photos")).unwrap();
        let again = make_folder(&test.at("photos"));
        assert!(matches!(again, Err(ActionError::AlreadyExists(_))));
    }

    #[test]
    fn makes_an_empty_file_but_never_wipes_one() {
        let test = TestFolder::new("make-file");
        let notes = test.at("notes.txt");
        make_file(&notes).unwrap();
        assert_eq!(fs::read_to_string(&notes).unwrap(), "");

        fs::write(&notes, "keep me").unwrap();
        assert!(matches!(
            make_file(&notes),
            Err(ActionError::AlreadyExists(_))
        ));
        assert_eq!(fs::read_to_string(&notes).unwrap(), "keep me");
    }

    #[test]
    fn reads_text_files() {
        let test = TestFolder::new("read");
        let notes = test.at("notes.txt");
        fs::write(&notes, "hello\nworld\n").unwrap();
        assert_eq!(read(&notes).unwrap(), "hello\nworld");
    }

    #[test]
    fn explains_why_a_read_failed() {
        let test = TestFolder::new("read-fail");
        fs::write(test.at("picture.bin"), [0xFF, 0xFE, 0x00]).unwrap();

        assert!(matches!(
            read(&test.at("missing.txt")),
            Err(ActionError::NotFound(_))
        ));
        assert!(matches!(
            read(&test.0.display().to_string()),
            Err(ActionError::IsAFolder(_))
        ));
        assert!(matches!(
            read(&test.at("picture.bin")),
            Err(ActionError::NotText(_))
        ));
    }

    #[test]
    fn lists_folders_first_and_hides_hidden_files() {
        let test = TestFolder::new("show");
        fs::write(test.at("b.txt"), "").unwrap();
        fs::write(test.at(".secret"), "").unwrap();
        fs::create_dir(test.at("zebra")).unwrap();

        let listing = show_files(&test.0).unwrap();
        assert_eq!(listing, "  📁 zebra/\n  📄 b.txt");
    }

    #[test]
    fn says_when_a_folder_is_empty() {
        let test = TestFolder::new("show-empty");
        assert_eq!(show_files(&test.0).unwrap(), "This folder is empty.");
    }

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
