//! Real commands for the Mac and Linux terminal (zsh or bash).

use std::path::Path;

use crate::command::{Command, opener};

pub(super) fn real(command: &Command) -> Option<String> {
    let text = match command {
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

    /// Lets the tests write `command.unix_real()`.
    trait UnixReal {
        fn unix_real(&self) -> Option<String>;
    }

    impl UnixReal for Command {
        fn unix_real(&self) -> Option<String> {
            real(self)
        }
    }

    #[test]
    fn shows_the_real_mkdir_command() {
        let cmd = Command::MakeFolder("photos".to_string());
        assert_eq!(cmd.unix_real(), Some("mkdir -p photos".to_string()));
    }

    #[test]
    fn real_commands_are_safe_to_paste() {
        let real = |name: &str| Command::Delete(name.to_string()).unix_real().unwrap();
        assert!(
            real("-n").starts_with("rm ./-n"),
            "a leading dash would look like an option"
        );
        assert!(real("a$b").starts_with("rm 'a$b'"));
        assert!(real("it's").starts_with(r"rm 'it'\''s'"));
    }

    #[test]
    fn real_commands_warn_that_rm_skips_the_trash() {
        let real = Command::Delete("a.txt".to_string()).unix_real().unwrap();
        assert!(real.contains("skips the Trash"));
    }

    #[test]
    fn real_commands_for_the_new_powers() {
        let real = |command: Command| command.unix_real().unwrap();
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
        assert_eq!(write.unix_real().unwrap(), "printf '%s\\n' -n > a.txt");
        let find = Command::FindText {
            text: "-n".to_string(),
            place: "docs".to_string(),
        };
        assert_eq!(find.unix_real().unwrap(), "grep -rni -e -n docs");
    }

    #[test]
    fn going_home_is_cd_squiggle() {
        assert_eq!(Command::GoHome.unix_real(), Some("cd ~".to_string()));
    }

    #[test]
    fn quotes_names_with_spaces() {
        let cmd = Command::MakeFolder("my photos".to_string());
        assert_eq!(cmd.unix_real(), Some("mkdir -p 'my photos'".to_string()));
    }
}
