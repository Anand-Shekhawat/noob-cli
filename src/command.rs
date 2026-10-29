//! Every phrase Noob understands becomes one of these `Command` values.
//!
//! An `enum` is a type that can be exactly one of several choices.
//! Some choices carry data, e.g. `MakeFolder` remembers the folder's name.
//! `Option<String>` means "maybe a name": `show files` or `show files photos`.

use std::path::Path;

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
    // Other
    Clear,
    /// `help` on its own, or `help copy` for one command.
    Help(Option<String>),
    Bye,
}

impl Command {
    /// The real terminal command that does the same job, so you learn it as you go.
    /// (Worked out before the command runs, because a few depend on what's there.)
    pub fn real_command(&self) -> Option<String> {
        let text = match self {
            Command::WhereAmI => "pwd".to_string(),
            Command::GoInto(name) => format!("cd {}", quote(name)),
            Command::GoBack => "cd ..".to_string(),
            Command::GoHome => "cd ~".to_string(),
            Command::ShowFiles(folder) => with_folder("ls", folder),
            Command::ShowAllFiles(folder) => with_folder("ls -a", folder),
            Command::ShowDetails(folder) => with_folder("ls -l", folder),
            Command::ShowTree(folder) => with_folder("ls -R", folder),
            Command::ShowSize(name) => format!("du -sh {}", quote(name)),
            Command::Read(name) => format!("cat {}", quote(name)),
            Command::MakeFolder(name) => format!("mkdir -p {}", quote(name)),
            Command::MakeFile(name) => format!("touch {}", quote(name)),
            Command::Write { text, file } => format!("{} > {}", echo(text), quote(file)),
            Command::Add { text, file } => format!("{} >> {}", echo(text), quote(file)),
            Command::Open(name) => format!("{} {}", opener(), quote(name)),
            Command::Copy { from, to } => {
                let copy = if Path::new(from).is_dir() {
                    "cp -R"
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
                    "rm -r"
                } else {
                    "rm"
                };
                format!(
                    "{remove} {}   (careful: rm skips the Trash, so it's gone for good)",
                    quote(name)
                )
            }
            Command::FindFile(name) => format!("find . -iname {}", quote(&format!("*{name}*"))),
            // `-e` tells grep "the next word is what to look for", even if it starts with "-".
            Command::FindText { text, place } => {
                format!("grep -rni -e {} {}", quote_text(text), quote(place))
            }
            Command::Clear => "clear".to_string(),
            Command::Help(_) | Command::Bye => return None,
        };
        Some(text)
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

/// "ls", or "ls photos" when a folder was named.
fn with_folder(command: &str, folder: &Option<String>) -> String {
    match folder {
        Some(folder) => format!("{command} {}", quote(folder)),
        None => command.to_string(),
    }
}

/// `echo` prints text. Text starting with "-" would look like an option to echo,
/// so then the real command uses `printf`, which never mixes them up.
fn echo(text: &str) -> String {
    if text.starts_with('-') {
        format!("printf '%s\\n' {}", quote_text(text))
    } else {
        format!("echo {}", quote_text(text))
    }
}

/// Makes text (not a file name) safe to paste: quoted if needed, but never given "./".
fn quote_text(text: &str) -> String {
    let plain = !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._/-".contains(c));
    if plain {
        text.to_string()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

/// Makes a name safe to paste into a real terminal.
fn quote(name: &str) -> String {
    // A name starting with "-" would look like an option (`rm -n`), so write it as "./-n".
    let name = if name.starts_with('-') {
        format!("./{name}")
    } else {
        name.to_string()
    };
    let plain = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "._/-".contains(c));
    if plain {
        return name;
    }
    // Inside 'single quotes' the shell treats everything literally,
    // except a ' itself, which is written as '\''.
    format!("'{}'", name.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_the_real_mkdir_command() {
        let cmd = Command::MakeFolder("photos".to_string());
        assert_eq!(cmd.real_command(), Some("mkdir -p photos".to_string()));
    }

    #[test]
    fn real_commands_are_safe_to_paste() {
        let real = |name: &str| Command::Delete(name.to_string()).real_command().unwrap();
        assert!(
            real("-n").starts_with("rm ./-n"),
            "a leading dash would look like an option"
        );
        assert!(real("a$b").starts_with("rm 'a$b'"));
        assert!(real("it's").starts_with(r"rm 'it'\''s'"));
    }

    #[test]
    fn real_commands_warn_that_rm_skips_the_trash() {
        let real = Command::Delete("a.txt".to_string()).real_command().unwrap();
        assert!(real.contains("skips the Trash"));
    }

    #[test]
    fn real_commands_for_the_new_powers() {
        let real = |command: Command| command.real_command().unwrap();
        assert_eq!(real(Command::ShowFiles(None)), "ls");
        assert_eq!(
            real(Command::ShowDetails(Some("docs".to_string()))),
            "ls -l docs"
        );
        assert_eq!(
            real(Command::FindFile("cat".to_string())),
            "find . -iname '*cat*'"
        );
        let add = Command::Add {
            text: "buy milk".to_string(),
            file: "list.txt".to_string(),
        };
        assert_eq!(real(add), "echo 'buy milk' >> list.txt");
    }

    #[test]
    fn text_starting_with_a_dash_is_never_mistaken_for_an_option() {
        let write = Command::Write {
            text: "-n".to_string(),
            file: "a.txt".to_string(),
        };
        assert_eq!(write.real_command().unwrap(), "printf '%s\\n' -n > a.txt");
        let find = Command::FindText {
            text: "-n".to_string(),
            place: "docs".to_string(),
        };
        assert_eq!(find.real_command().unwrap(), "grep -rni -e -n docs");
    }

    #[test]
    fn going_places_only_works_in_the_prompt() {
        assert!(Command::GoBack.prompt_only());
        assert!(Command::GoInto("photos".to_string()).prompt_only());
        assert!(!Command::ShowFiles(None).prompt_only());
        assert_eq!(Command::GoHome.real_command(), Some("cd ~".to_string()));
    }

    #[test]
    fn quotes_names_with_spaces() {
        let cmd = Command::MakeFolder("my photos".to_string());
        assert_eq!(cmd.real_command(), Some("mkdir -p 'my photos'".to_string()));
    }
}
