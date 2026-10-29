//! Making things, looking inside files, writing to them and opening them.

use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process;

use super::Reply;
use super::errors::{ActionError, friendly};
use super::paths::{existing, folder_of, is_shortcut, taken};
use super::words::{human_size, plural};

/// `read` refuses files bigger than this; they're better opened in an app.
const MAX_READ_BYTES: u64 = 10 * 1024 * 1024;
/// `read` shows at most this many lines.
const MAX_READ_LINES: usize = 200;

pub(super) fn make_folder(name: &str) -> Result<Reply, ActionError> {
    let path = Path::new(name);
    if taken(path) {
        return Err(ActionError::AlreadyExists(name.to_string()));
    }
    fs::create_dir_all(path).map_err(|error| friendly(error, name))?;
    Ok(Reply::Done(format!("Made folder \"{name}\"")))
}

pub(super) fn make_file(name: &str) -> Result<Reply, ActionError> {
    check_folder_exists(Path::new(name))?;
    // `create_new` refuses to touch a file that already exists, so nothing gets wiped.
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(name)
        .map_err(|error| friendly(error, name))?;
    Ok(Reply::Done(format!("Made file \"{name}\"")))
}

pub(super) fn read(name: &str) -> Result<Reply, ActionError> {
    let path = existing(name)?;
    // `metadata` follows a shortcut to the real file, which is fine for just reading.
    let info = fs::metadata(path).map_err(|error| friendly(error, name))?;
    if info.is_dir() {
        return Err(ActionError::IsAFolder(name.to_string()));
    }
    if !info.is_file() {
        // Devices like /dev/zero never end, and pipes can wait forever.
        return Err(ActionError::NotAFile(name.to_string()));
    }
    if info.len() > MAX_READ_BYTES {
        return Err(ActionError::Explained(format!(
            "\"{name}\" is very big ({}). Try: open {name}",
            human_size(info.len())
        )));
    }
    let text = fs::read_to_string(path).map_err(|error| friendly(error, name))?;
    if text.trim().is_empty() {
        return Ok(Reply::Show(format!("\"{name}\" is empty.")));
    }
    let lines: Vec<&str> = text.trim_end().lines().collect();
    if lines.len() <= MAX_READ_LINES {
        return Ok(Reply::Show(lines.join("\n")));
    }
    let more = (lines.len() - MAX_READ_LINES) as u64;
    Ok(Reply::Show(format!(
        "{}\n… and {}. To see it all, try: open {name}",
        lines[..MAX_READ_LINES].join("\n"),
        plural(more, "more line")
    )))
}

/// The question to ask before `write` replaces text that's already there.
pub(super) fn replace_question(file: &str) -> Option<String> {
    let has_text = fs::symlink_metadata(file).is_ok_and(|info| info.is_file() && info.len() > 0);
    has_text.then(|| format!("\"{file}\" already has text in it. Replace all of it?"))
}

pub(super) fn write(text: &str, file: &str) -> Result<Reply, ActionError> {
    check_writable(file)?;
    fs::write(file, format!("{text}\n")).map_err(|error| friendly(error, file))?;
    Ok(Reply::Done(format!("Wrote your text into \"{file}\"")))
}

pub(super) fn add(text: &str, file: &str) -> Result<Reply, ActionError> {
    check_writable(file)?;
    let existed = taken(Path::new(file));
    // If the last line has no "Enter" at its end, add one first so our text starts a new line.
    let line = if existed && !ends_with_new_line(file)? {
        format!("\n{text}\n")
    } else {
        format!("{text}\n")
    };
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(file)
        .and_then(|mut opened| opened.write_all(line.as_bytes()))
        .map_err(|error| friendly(error, file))?;
    let message = if existed {
        format!("Added your text to the end of \"{file}\"")
    } else {
        format!("Made \"{file}\" and added your text")
    };
    Ok(Reply::Done(message))
}

/// Writing needs a normal file (or nothing yet) in a folder that exists.
fn check_writable(file: &str) -> Result<(), ActionError> {
    let path = Path::new(file);
    if is_shortcut(path) {
        return Err(ActionError::Shortcut(file.to_string()));
    }
    if let Ok(info) = fs::symlink_metadata(path) {
        if info.is_dir() {
            return Err(ActionError::IsAFolder(file.to_string()));
        }
        if !info.is_file() {
            return Err(ActionError::NotAFile(file.to_string()));
        }
    }
    check_folder_exists(path)
}

fn check_folder_exists(path: &Path) -> Result<(), ActionError> {
    let folder = folder_of(path);
    if folder.is_dir() {
        Ok(())
    } else {
        Err(ActionError::NotFound(folder.display().to_string()))
    }
}

fn ends_with_new_line(file: &str) -> Result<bool, ActionError> {
    let mut opened = fs::File::open(file).map_err(|error| friendly(error, file))?;
    let size = opened
        .metadata()
        .map_err(|error| friendly(error, file))?
        .len();
    if size == 0 {
        return Ok(true);
    }
    // Jump straight to the last letter instead of reading the whole file.
    let mut last = [0u8; 1];
    opened
        .seek(SeekFrom::End(-1))
        .and_then(|_| opened.read_exact(&mut last))
        .map_err(|error| friendly(error, file))?;
    Ok(last[0] == b'\n')
}

/// Opens a file or folder in its usual app, the way double-clicking it would.
pub(super) fn open(name: &str) -> Result<Reply, ActionError> {
    let path = existing(name)?;
    // A name starting with "-" would look like an option to the opener, so write it as "./-name".
    let path = if name.starts_with('-') {
        Path::new(".").join(path)
    } else {
        path.to_path_buf()
    };
    let program = crate::command::opener();
    let status = process::Command::new(program)
        .arg(&path)
        .status()
        .map_err(|error| ActionError::Explained(format!("I couldn't start {program}: {error}")))?;
    // Windows' explorer often reports "failed" even when it opened the file, so only
    // trust the answer on other computers.
    if !status.success() && !cfg!(windows) {
        return Err(ActionError::Explained(format!(
            "{program} couldn't open \"{name}\"."
        )));
    }
    Ok(Reply::Done(format!("Opened \"{name}\"")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::test_support::{TestFolder, make_pipe};

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
    fn a_file_in_a_missing_folder_blames_the_folder() {
        let test = TestFolder::new("make-file-lost");
        let result = make_file(&test.at("nope/notes.txt"));
        assert!(matches!(result, Err(ActionError::NotFound(name)) if name == test.at("nope")));
    }

    #[test]
    fn reads_text_files() {
        let test = TestFolder::new("read");
        let notes = test.at("notes.txt");
        fs::write(&notes, "hello\nworld\n").unwrap();
        assert_eq!(
            read(&notes).unwrap(),
            Reply::Show("hello\nworld".to_string())
        );
    }

    #[test]
    fn long_files_are_cut_short_with_a_tip() {
        let test = TestFolder::new("read-long");
        let notes = test.at("long.txt");
        let text: String = (1..=250).map(|number| format!("line {number}\n")).collect();
        fs::write(&notes, text).unwrap();

        let Reply::Show(shown) = read(&notes).unwrap() else {
            panic!("expected text")
        };
        assert!(shown.contains("line 200\n… and 50 more lines."), "{shown}");
        assert!(!shown.contains("line 201"));
    }

    #[test]
    fn explains_why_a_read_failed() {
        let test = TestFolder::new("read-fail");
        fs::write(test.at("picture.bin"), [0xFF, 0xFE, 0x00]).unwrap();

        assert!(matches!(
            read(&test.at("missing.txt")),
            Err(ActionError::NotFound(_))
        ));
        assert!(matches!(read(&test.path()), Err(ActionError::IsAFolder(_))));
        assert!(matches!(
            read(&test.at("picture.bin")),
            Err(ActionError::NotText(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn will_not_read_or_write_pipes_that_could_wait_forever() {
        let test = TestFolder::new("read-pipe");
        let pipe = make_pipe(&test, "pipe");
        assert!(matches!(read(&pipe), Err(ActionError::NotAFile(_))));
        assert!(matches!(add("hi", &pipe), Err(ActionError::NotAFile(_))));
    }

    #[test]
    fn write_replaces_the_text_and_asks_first_only_if_there_was_some() {
        let test = TestFolder::new("write");
        let notes = test.at("notes.txt");
        assert_eq!(replace_question(&notes), None, "nothing to lose yet");

        write("first", &notes).unwrap();
        assert!(replace_question(&notes).is_some());
        write("second", &notes).unwrap();
        assert_eq!(fs::read_to_string(&notes).unwrap(), "second\n");
    }

    #[test]
    fn add_puts_text_on_a_new_line_at_the_end() {
        let test = TestFolder::new("add");
        let notes = test.at("notes.txt");
        fs::write(&notes, "buy milk").unwrap(); // no "Enter" at the end

        add("buy eggs", &notes).unwrap();
        assert_eq!(fs::read_to_string(&notes).unwrap(), "buy milk\nbuy eggs\n");

        let new = test.at("new.txt");
        assert!(matches!(add("hi", &new), Ok(Reply::Done(message)) if message.starts_with("Made")));
        assert_eq!(fs::read_to_string(&new).unwrap(), "hi\n");
    }

    #[cfg(unix)]
    #[test]
    fn will_not_write_through_shortcuts() {
        let test = TestFolder::new("write-link");
        fs::write(test.at("real.txt"), "keep").unwrap();
        std::os::unix::fs::symlink(test.at("real.txt"), test.at("link")).unwrap();
        std::os::unix::fs::symlink(test.at("nowhere.txt"), test.at("broken")).unwrap();

        assert!(matches!(
            write("x", &test.at("link")),
            Err(ActionError::Shortcut(_))
        ));
        assert!(matches!(
            add("x", &test.at("broken")),
            Err(ActionError::Shortcut(_))
        ));
        assert_eq!(fs::read_to_string(test.at("real.txt")).unwrap(), "keep");
        assert!(
            !Path::new(&test.at("nowhere.txt")).exists(),
            "no surprise file was made"
        );
    }

    #[test]
    fn writing_needs_a_file_in_a_folder_that_exists() {
        let test = TestFolder::new("write-fail");
        assert!(matches!(
            write("hi", &test.path()),
            Err(ActionError::IsAFolder(_))
        ));
        let lost = write("hi", &test.at("nope/notes.txt"));
        assert!(matches!(lost, Err(ActionError::NotFound(name)) if name == test.at("nope")));
    }

    #[test]
    fn opening_something_missing_is_explained() {
        // Opening something real would launch an app, so tests only check the missing case.
        let test = TestFolder::new("open");
        assert!(matches!(
            open(&test.at("ghost.txt")),
            Err(ActionError::NotFound(_))
        ));
    }
}
