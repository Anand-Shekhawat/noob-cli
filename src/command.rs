//! Every phrase Noob understands becomes one of these `Command` values.
//!
//! An `enum` is a type that can be exactly one of several choices.
//! Some choices carry data, e.g. `MakeFolder` remembers the folder's name.

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Command {
    WhereAmI,
    GoInto(String),
    GoBack,
    GoHome,
    ShowFiles,
    MakeFolder(String),
    MakeFile(String),
    Read(String),
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
    Clear,
    /// `help` on its own, or `help copy` for one command.
    Help(Option<String>),
    Bye,
}

impl Command {
    /// The real terminal command that does the same job, so you learn it as you go.
    pub fn real_command(&self) -> Option<String> {
        let text = match self {
            Command::WhereAmI => "pwd".to_string(),
            Command::GoInto(name) => format!("cd {}", quote(name)),
            Command::GoBack => "cd ..".to_string(),
            Command::GoHome => "cd ~".to_string(),
            Command::ShowFiles => "ls".to_string(),
            Command::MakeFolder(name) => format!("mkdir -p {}", quote(name)),
            Command::MakeFile(name) => format!("touch {}", quote(name)),
            Command::Read(name) => format!("cat {}", quote(name)),
            Command::Copy { from, to } => format!("cp {} {}", quote(from), quote(to)),
            Command::Move { from, to } | Command::Rename { from, to } => {
                format!("mv {} {}", quote(from), quote(to))
            }
            Command::Delete(name) => format!("rm {0}   (for a folder: rmdir {0})", quote(name)),
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

    /// Risky commands return a question that must be answered "yes" before they run.
    pub fn confirmation_question(&self) -> Option<String> {
        match self {
            Command::Delete(name) => Some(format!("Delete \"{name}\"? This can't be undone.")),
            _ => None,
        }
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
    fn going_places_only_works_in_the_prompt() {
        assert!(Command::GoBack.prompt_only());
        assert!(Command::GoInto("photos".to_string()).prompt_only());
        assert!(!Command::ShowFiles.prompt_only());
        assert_eq!(Command::GoHome.real_command(), Some("cd ~".to_string()));
    }

    #[test]
    fn quotes_names_with_spaces() {
        let cmd = Command::MakeFolder("my photos".to_string());
        assert_eq!(cmd.real_command(), Some("mkdir -p 'my photos'".to_string()));
    }

    #[test]
    fn only_delete_asks_for_confirmation() {
        let delete = Command::Delete("a.txt".to_string());
        let make = Command::MakeFile("a.txt".to_string());
        assert!(delete.confirmation_question().unwrap().contains("a.txt"));
        assert_eq!(make.confirmation_question(), None);
    }
}
