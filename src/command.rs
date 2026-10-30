//! Every phrase Noob understands becomes one of these `Command` values.
//!
//! An `enum` is a type that can be exactly one of several choices.
//! Some choices carry data, e.g. `MakeFolder` remembers the folder's name.
//! `Option<String>` means "maybe a name": `show files` or `show files photos`.

use crate::real::{self, Shell};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    // Getting around
    WhereAmI,
    GoInto(String),
    GoBack,
    GoHome,
    // Looking at things
    ShowFiles(Option<String>),
    ShowAllFiles(Option<String>),
    ShowDetails(Option<String>),
    ShowTree(Option<String>),
    ShowSize(String),
    Read(String),
    // Making and changing
    MakeFolder(String),
    MakeFile(String),
    Write {
        text: String,
        file: String,
    },
    Add {
        text: String,
        file: String,
    },
    Open(String),
    // Copying, moving, deleting
    Copy {
        from: String,
        to: String,
    },
    Move {
        from: String,
        to: String,
    },
    Rename {
        from: String,
        to: String,
    },
    Delete(String),
    // Finding
    FindFile(String),
    FindText {
        text: String,
        place: String,
    },
    // Installing tools and apps (the work happens in `tools`)
    Tool(ToolCommand),
    // Other
    Version,
    Clear,
    /// `help` on its own, or `help copy` for one command.
    Help(Option<String>),
    Bye,
}

/// Commands for installing tools and apps, kept together so `tools` can handle all of
/// them, and the compiler can check that none is forgotten.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ToolCommand {
    Install(String),
    Uninstall(String),
    UpdateAll,
    ShowTools,
    ShowInstalled,
    IsInstalled(String),
    SearchTools(String),
}

impl Command {
    /// The real terminal command that does the same job, so you learn it as you go:
    /// a Mac or Linux command, or a PowerShell one on Windows.
    /// (Worked out before the command runs, because a few depend on what's there.)
    pub fn real_command(&self) -> Option<String> {
        real::for_shell(self, Shell::current())
    }

    /// Moving around only works inside the `noob>` prompt. In the normal terminal,
    /// Noob is a separate program, and a program can't move the terminal that started it.
    pub fn prompt_only(&self) -> bool {
        matches!(self, Command::GoInto(_) | Command::GoBack | Command::GoHome)
    }
}

/// The program that opens a file in its usual app on this kind of computer.
pub fn opener() -> &'static str {
    if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn going_places_only_works_in_the_prompt() {
        assert!(Command::GoBack.prompt_only());
        assert!(Command::GoInto("photos".to_string()).prompt_only());
        assert!(!Command::ShowFiles(None).prompt_only());
    }
}
