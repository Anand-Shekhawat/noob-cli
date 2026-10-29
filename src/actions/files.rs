//! Making things and looking inside files.

use std::fs;
use std::path::Path;

use super::Reply;
use super::errors::{ActionError, friendly};
use super::paths::taken;

pub(super) fn make_folder(name: &str) -> Result<Reply, ActionError> {
    let path = Path::new(name);
    if taken(path) {
        return Err(ActionError::AlreadyExists(name.to_string()));
    }
    fs::create_dir_all(path).map_err(|error| friendly(error, name))?;
    Ok(Reply::Done(format!("Made folder \"{name}\"")))
}

pub(super) fn make_file(name: &str) -> Result<Reply, ActionError> {
    // `create_new` refuses to touch a file that already exists, so nothing gets wiped.
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(name)
        .map_err(|error| friendly(error, name))?;
    Ok(Reply::Done(format!("Made file \"{name}\"")))
}

pub(super) fn read(name: &str) -> Result<Reply, ActionError> {
    if Path::new(name).is_dir() {
        return Err(ActionError::IsAFolder(name.to_string()));
    }
    let text = fs::read_to_string(name).map_err(|error| friendly(error, name))?;
    if text.trim().is_empty() {
        return Ok(Reply::Show(format!("\"{name}\" is empty.")));
    }
    Ok(Reply::Show(text.trim_end().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::test_support::TestFolder;

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
        assert_eq!(
            read(&notes).unwrap(),
            Reply::Show("hello\nworld".to_string())
        );
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
}
