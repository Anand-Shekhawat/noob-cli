//! Everything that can go wrong, in words a beginner understands.

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum ActionError {
    NotFound(String),
    AlreadyExists(String),
    FolderNotEmpty(String),
    IsAFolder(String),
    NotAFolder(String),
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
            ActionError::NotFound(name) => write!(
                f,
                "I couldn't find \"{name}\". Type `show files` to see what's here."
            ),
            ActionError::AlreadyExists(name) => write!(
                f,
                "\"{name}\" already exists. Pick another name, or delete it first."
            ),
            ActionError::FolderNotEmpty(name) => write!(
                f,
                "\"{name}\" is not empty (it may hold hidden files). \
                 For safety, Noob only deletes empty folders."
            ),
            ActionError::IsAFolder(name) => write!(
                f,
                "\"{name}\" is a folder. This only works with files for now."
            ),
            ActionError::NotAFolder(name) => write!(f, "\"{name}\" is a file, not a folder."),
            ActionError::NotText(name) => {
                write!(f, "\"{name}\" isn't a text file, so I can't show it.")
            }
            ActionError::NotAllowed(name) => write!(
                f,
                "Your computer won't let me touch \"{name}\" (permission denied)."
            ),
            ActionError::IntoItself(name) => write!(f, "I can't put \"{name}\" inside itself."),
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
pub(super) fn friendly(error: io::Error, name: &str) -> ActionError {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turns_system_errors_into_friendly_ones() {
        let not_found = io::Error::from(io::ErrorKind::NotFound);
        assert!(
            matches!(friendly(not_found, "a.txt"), ActionError::NotFound(name) if name == "a.txt")
        );

        let denied = io::Error::from(io::ErrorKind::PermissionDenied);
        assert!(matches!(
            friendly(denied, "a.txt"),
            ActionError::NotAllowed(_)
        ));
    }
}
