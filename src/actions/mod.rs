//! Does the real work for each command. Each file in this folder handles one kind of job.

mod errors;
mod files;
mod finding;
mod listing;
mod moving;
mod navigate;
mod paths;
#[cfg(test)]
mod test_support;
mod walk;
mod words;

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

/// Risky commands return a question that must be answered "yes" before they run.
pub fn question(command: &Command) -> Option<String> {
    match command {
        Command::Delete(name) => Some(moving::delete_question(name)),
        Command::Write { file, .. } => files::replace_question(file),
        _ => None,
    }
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
/// Runs one command. `ask` puts a yes/no question on screen, for commands that need
/// to ask in the middle of their work (like install).
pub fn execute(command: &Command, ask: &mut dyn FnMut(&str) -> bool) -> Result<Reply, ActionError> {
    if crate::tools::is_tool_command(command) {
        return crate::tools::run(command, ask);
    }
    match command {
        Command::WhereAmI => navigate::where_am_i(),
        Command::GoInto(name) => navigate::go_into(name),
        Command::GoBack => navigate::go_back(),
        Command::GoHome => navigate::go_home(),
        Command::ShowFiles(folder) => listing::show_files(here_or(folder), false),
        Command::ShowAllFiles(folder) => listing::show_files(here_or(folder), true),
        Command::ShowDetails(folder) => listing::show_details(here_or(folder)),
        Command::ShowTree(folder) => listing::show_tree(here_or(folder)),
        Command::ShowSize(name) => listing::show_size(name),
        Command::Read(name) => files::read(name),
        Command::MakeFolder(name) => files::make_folder(name),
        Command::MakeFile(name) => files::make_file(name),
        Command::Write { text, file } => files::write(text, file),
        Command::Add { text, file } => files::add(text, file),
        Command::Open(name) => files::open(name),
        Command::Copy { from, to } => moving::copy(from, to),
        Command::Move { from, to } => moving::move_into(from, to),
        Command::Rename { from, to } => moving::rename(from, to),
        Command::Delete(name) => moving::delete(name),
        Command::FindFile(name) => finding::find_file(name),
        Command::FindText { text, place } => finding::find_text(text, place),
        Command::Clear => Ok(Reply::Show(CLEAR_SCREEN.to_string())),
        Command::Help(None) => Ok(Reply::Show(help::everything())),
        Command::Help(Some(topic)) => help::about(topic)
            .map(Reply::Show)
            .ok_or_else(|| ActionError::NoHelp(topic.clone())),
        Command::Version => Ok(Reply::Show(format!("Noob {}", env!("CARGO_PKG_VERSION")))),
        Command::Bye => Ok(Reply::Show("Bye! See you next time.".to_string())),
        // Handled by `tools` above.
        Command::Install(_)
        | Command::Uninstall(_)
        | Command::UpdateAll
        | Command::ShowTools
        | Command::ShowInstalled
        | Command::IsInstalled(_)
        | Command::SearchTools(_) => {
            Err(ActionError::Explained("That's a tool command.".to_string()))
        }
    }
}

/// The folder that was named, or the current folder (".") if none was.
fn here_or(folder: &Option<String>) -> &Path {
    folder.as_deref().map_or(Path::new("."), Path::new)
}
