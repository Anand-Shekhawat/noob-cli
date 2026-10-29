//! Does the real work for each command. Each file in this folder handles one kind of job.

mod errors;
mod files;
mod listing;
mod moving;
mod navigate;
mod paths;
#[cfg(test)]
mod test_support;

pub use errors::ActionError;
pub use paths::short;

use std::path::Path;

use crate::command::Command;
use crate::help;

/// The special characters that tell a terminal to wipe the screen.
const CLEAR_SCREEN: &str = "\x1B[2J\x1B[1;1H";

/// What to show after a command worked.
#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    /// Something was done. Shown in green, with a ✓.
    Done(String),
    /// Something to look at, like a file's contents. Shown as it is.
    Show(String),
}

/// Checks that a risky command can actually run, so we never ask
/// "are you sure?" about something that is going to fail anyway.
pub fn check_ready(command: &Command) -> Result<(), ActionError> {
    match command {
        Command::Delete(name) => moving::check_delete(name),
        _ => Ok(()),
    }
}

/// Runs one command.
pub fn execute(command: &Command) -> Result<Reply, ActionError> {
    match command {
        Command::WhereAmI => navigate::where_am_i(),
        Command::GoInto(name) => navigate::go_into(name),
        Command::GoBack => navigate::go_back(),
        Command::GoHome => navigate::go_home(),
        Command::ShowFiles => listing::show_files(Path::new(".")),
        Command::Read(name) => files::read(name),
        Command::MakeFolder(name) => files::make_folder(name),
        Command::MakeFile(name) => files::make_file(name),
        Command::Copy { from, to } => moving::copy(from, to),
        Command::Move { from, to } => moving::move_into(from, to),
        Command::Rename { from, to } => moving::rename(from, to),
        Command::Delete(name) => moving::delete(name),
        Command::Clear => Ok(Reply::Show(CLEAR_SCREEN.to_string())),
        Command::Help(topic) => Ok(Reply::Show(help::text(topic.as_deref()))),
        Command::Bye => Ok(Reply::Show("Bye! See you next time.".to_string())),
    }
}
