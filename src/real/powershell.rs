//! Real commands for PowerShell, the terminal on Windows.
//!
//! PowerShell understands some Mac-style names (`ls`, `cp`, `mv`, `rm`, `cat`) as
//! shortcuts for its own commands, so those are used where they exist.

use std::path::Path;

use crate::command::Command;

pub(super) fn real(command: &Command) -> Option<String> {
    let text = match command {
        Command::WhereAmI => "pwd".to_string(),
        Command::GoInto(name) => format!("cd {}", quote(name)),
        Command::GoBack => "cd ..".to_string(),
        Command::GoHome => "cd ~".to_string(),
        Command::ShowFiles(folder) | Command::ShowDetails(folder) => with_folder("ls", folder),
        Command::ShowAllFiles(folder) => match folder {
            Some(folder) => format!("ls {} -Force", quote(folder)),
            None => "ls -Force".to_string(),
        },
        Command::ShowTree(folder) => match folder {
            Some(folder) => format!("tree {} /f", quote(folder)),
            None => "tree /f".to_string(),
        },
        Command::ShowSize(name) => format!(
            "(ls {} -Recurse -File | Measure-Object Length -Sum).Sum",
            quote(name)
        ),
        Command::Read(name) => format!("cat {}", quote(name)),
        // PowerShell's mkdir makes any missing folders on the way, like `mkdir -p`.
        Command::MakeFolder(name) => format!("mkdir {}", quote(name)),
        Command::MakeFile(name) => format!("New-Item {}", quote(name)),
        Command::Write { text, file } => {
            format!("Set-Content {} {}", quote(file), quote_text(text))
        }
        Command::Add { text, file } => format!("Add-Content {} {}", quote(file), quote_text(text)),
        Command::Open(name) => format!("start {}", quote(name)),
        Command::Copy { from, to } => {
            let copy = if Path::new(from).is_dir() {
                "cp -Recurse"
            } else {
                "cp"
            };
            format!("{copy} {} {}", quote(from), quote(to))
        }
        Command::Move { from, to } | Command::Rename { from, to } => {
            format!("mv {} {}", quote(from), quote(to))
        }
        Command::Delete(name) => {
            let remove = if Path::new(name).is_dir() {
                "rm -Recurse"
            } else {
                "rm"
            };
            format!(
                "{remove} {}   (careful: rm skips the Recycle Bin, so it's gone for good)",
                quote(name)
            )
        }
        Command::FindFile(name) => {
            format!("ls -Recurse -Filter {}", quote_text(&format!("*{name}*")))
        }
        // `-SimpleMatch` means "look for exactly this text", not a pattern.
        Command::FindText { text, place } => {
            if Path::new(place).is_dir() {
                format!(
                    "ls {} -Recurse -File | Select-String -SimpleMatch {}",
                    quote(place),
                    quote_text(text)
                )
            } else {
                format!(
                    "Select-String -SimpleMatch {} {}",
                    quote_text(text),
                    quote(place)
                )
            }
        }
        Command::Clear => "clear".to_string(),
        Command::Help(_) | Command::Bye => return None,
    };
    Some(text)
}

/// "ls", or "ls photos" when a folder was named.
fn with_folder(command: &str, folder: &Option<String>) -> String {
    match folder {
        Some(folder) => format!("{command} {}", quote(folder)),
        None => command.to_string(),
    }
}

fn is_plain(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-\\/:".contains(c))
}

/// Text (not a file name). Text starting with "-" would look like an option, so it's quoted.
fn quote_text(text: &str) -> String {
    if is_plain(text) && !text.starts_with('-') {
        text.to_string()
    } else {
        single_quoted(text)
    }
}

/// Makes a name safe to paste into PowerShell.
fn quote(name: &str) -> String {
    // A name starting with "-" would look like an option, so write it as ".\-n".
    let name = if name.starts_with('-') {
        format!(".\\{name}")
    } else {
        name.to_string()
    };
    if is_plain(&name) {
        name
    } else {
        single_quoted(&name)
    }
}

/// Inside 'single quotes' PowerShell takes everything literally, except a ' itself,
/// which is written twice: 'it''s'.
fn single_quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn real_for(command: Command) -> String {
        real(&command).unwrap()
    }

    fn name(text: &str) -> String {
        text.to_string()
    }

    #[test]
    fn uses_powershell_commands() {
        assert_eq!(
            real_for(Command::MakeFolder(name("photos"))),
            "mkdir photos"
        );
        assert_eq!(
            real_for(Command::MakeFile(name("notes.txt"))),
            "New-Item notes.txt"
        );
        assert_eq!(real_for(Command::ShowAllFiles(None)), "ls -Force");
        let add = Command::Add {
            text: name("buy milk"),
            file: name("list.txt"),
        };
        assert_eq!(real_for(add), "Add-Content list.txt 'buy milk'");
    }

    #[test]
    fn says_recycle_bin_not_trash() {
        assert!(real_for(Command::Delete(name("a.txt"))).contains("Recycle Bin"));
    }

    #[test]
    fn quotes_the_powershell_way() {
        assert_eq!(
            real_for(Command::Read(name("it's mine.txt"))),
            "cat 'it''s mine.txt'"
        );
        assert_eq!(real_for(Command::Read(name("-n"))), "cat .\\-n");
        let write = Command::Write {
            text: name("-n"),
            file: name("a.txt"),
        };
        assert_eq!(real_for(write), "Set-Content a.txt '-n'");
    }

    #[test]
    fn finds_text_with_a_simple_match() {
        let find = Command::FindText {
            text: name("hello"),
            place: name("notes.txt"),
        };
        assert_eq!(real_for(find), "Select-String -SimpleMatch hello notes.txt");
    }
}
