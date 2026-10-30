//! The real terminal command that matches each Noob command, so you learn it as you go.
//!
//! Macs and Linux use one family of commands (`ls`, `cp`, `mkdir -p`).
//! Windows uses PowerShell, which has its own (`New-Item`, `Copy-Item`, `Select-String`).

mod powershell;
mod unix;

use crate::command::Command;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    /// The terminal on a Mac or Linux computer (zsh or bash).
    Unix,
    /// The terminal on Windows.
    PowerShell,
}

impl Shell {
    pub fn current() -> Shell {
        if cfg!(windows) {
            Shell::PowerShell
        } else {
            Shell::Unix
        }
    }
}

pub fn for_shell(command: &Command, shell: Shell) -> Option<String> {
    match shell {
        Shell::Unix => unix::real(command),
        Shell::PowerShell => powershell::real(command),
    }
}
