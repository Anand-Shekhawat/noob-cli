//! The Noob bar: plain English in, a real command typed into your shell out.
//!
//! You type `make folder photos` into the bar under the terminal. Noob works out
//! the real command (`mkdir -p photos`), writes it into the shell's prompt, and
//! stops. You read it, and press Enter yourself to run it. That way you always
//! see, and slowly learn, what really happens.
//!
//! This file only *decides* what to do; it never touches the shell. The window
//! carries the decision out. Deciding is kept separate so it's easy to test.
//!
//! Three things are different from just pasting Noob's "real command":
//!   - Deleting must stay safe. `rm` erases for good, so delete uses the Trash
//!     command when the computer has one, and otherwise Noob does it itself.
//!   - A few phrases have no real command (`help`): Noob answers in the bar.
//!   - Risky phrases (delete, replacing a file's text, installing) ask first.

use std::path::Path;

use crate::command::{Command, ToolCommand};
use crate::engine::{self, Line, Next, Place};
use crate::real::{self, Shell};

/// What this computer offers, found out by the window (so the logic here stays testable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Setup {
    pub shell: Shell,
    /// The command that sends things to the Trash, like `trash` on a Mac or
    /// `gio trash` on Linux. `None` when there isn't one.
    pub safe_delete: Option<String>,
    /// Whether Homebrew (`brew`) is installed, which Mac and Linux installs need.
    pub has_homebrew: bool,
}

impl Setup {
    /// Looks at this computer: which shell, whether there's a Trash command,
    /// and whether Homebrew is installed.
    pub fn detect() -> Setup {
        let safe_delete = if cfg!(target_os = "macos") {
            // Macs have had a `trash` command since macOS 15.
            Path::new("/usr/bin/trash")
                .exists()
                .then(|| "trash".to_string())
        } else if cfg!(target_os = "linux") {
            on_path("gio").then(|| "gio trash".to_string())
        } else {
            // Windows: Noob sends things to the Recycle Bin itself.
            None
        };
        Setup {
            shell: Shell::current(),
            safe_delete,
            has_homebrew: has_homebrew(),
        }
    }
}

/// Homebrew lives in one of a few known places. An app opened from the Dock
/// doesn't see the folders the shell adds to its search list, so look there too.
fn has_homebrew() -> bool {
    const PLACES: [&str; 3] = [
        "/opt/homebrew/bin/brew",
        "/usr/local/bin/brew",
        "/home/linuxbrew/.linuxbrew/bin/brew",
    ];
    on_path("brew") || PLACES.iter().any(|place| Path::new(place).exists())
}

/// Is there a program with this name in one of the folders in `PATH`?
fn on_path(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|folder| folder.join(program).is_file())
    })
}

/// What the bar should do with what you typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Ask this yes/no question first. On "no", nothing happens.
    pub ask: Option<String>,
    pub then: Then,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Then {
    /// Write `command` into the shell's prompt, for you to run with Enter.
    /// `about` says in plain words what it will do.
    Type { command: String, about: String },
    /// No safe real command for this: Noob does it itself (like `help`).
    RunHere(Command),
    /// Nothing to do; just show these lines (a hint, or a problem).
    Say(Vec<Line>),
}

/// Decides what to do with one line typed into the bar.
///
/// Names are read relative to the folder Noob is in, so the window first moves
/// Noob to the shell's folder.
pub fn plan(input: &str, setup: &Setup) -> Plan {
    let (question, command) = match engine::understand(input, Place::Window) {
        Next::Finished(outcome) => return say(outcome.lines),
        Next::Ask { question, command } => (Some(question), command),
        Next::Run(command) => (None, command),
    };
    let then = then_for(&command, setup);
    let ask = match (&then, &command) {
        // Nothing will happen, so there's nothing to ask about.
        (Then::Say(_), _) => None,
        (_, Command::Tool(tool)) => question.or_else(|| tool_question(tool)),
        _ => question,
    };
    Plan { ask, then }
}

fn say(lines: Vec<Line>) -> Plan {
    Plan {
        ask: None,
        then: Then::Say(lines),
    }
}

fn then_for(command: &Command, setup: &Setup) -> Then {
    let hint = |text: &str| Then::Say(vec![Line::Hint(text.to_string())]);
    match command {
        Command::Bye => hint("To leave, close the window, or type exit in the terminal."),
        Command::Delete(name) => match &setup.safe_delete {
            Some(trash) => Then::Type {
                command: format!("{trash} {}", real::quote_name(name, setup.shell)),
                about: about(command),
            },
            // Never `rm`: without a Trash command, Noob moves it to the Trash itself.
            None => Then::RunHere(command.clone()),
        },
        _ => match real::for_shell(command, setup.shell) {
            Some(real) if needs_homebrew(&real, setup) => {
                hint("Installing on this computer needs Homebrew first. Type: install homebrew")
            }
            // An invisible character (like a tab) can't be typed into a prompt
            // safely, and leaving it out would point at a different file.
            Some(real) if real.chars().any(char::is_control) => Then::Say(vec![Line::Problem(
                "That name has an invisible character in it, so Noob can't type it for you."
                    .to_string(),
            )]),
            Some(real) => Then::Type {
                command: real,
                about: about(command),
            },
            None => Then::RunHere(command.clone()),
        },
    }
}

/// A `brew` command on a computer that doesn't have Homebrew yet would only fail.
fn needs_homebrew(real: &str, setup: &Setup) -> bool {
    setup.shell == Shell::Unix && !setup.has_homebrew && real.starts_with("brew ")
}

/// Installing changes the computer, so it gets a question even though Enter is
/// still yours to press.
fn tool_question(tool: &ToolCommand) -> Option<String> {
    match tool {
        ToolCommand::Install(name) => Some(format!(
            "Install {name}? It downloads software from the internet."
        )),
        ToolCommand::Uninstall(name) => Some(format!("Remove {name} from this computer?")),
        ToolCommand::UpdateAll => {
            Some("Update everything you installed? It can take a while.".to_string())
        }
        ToolCommand::ShowTools
        | ToolCommand::ShowInstalled
        | ToolCommand::IsInstalled(_)
        | ToolCommand::SearchTools(_) => None,
    }
}

/// What a command does, in a few plain words.
fn about(command: &Command) -> String {
    let here = |folder: &Option<String>| match folder {
        Some(folder) => format!("in \"{folder}\""),
        None => "here".to_string(),
    };
    match command {
        Command::WhereAmI => "Shows the folder you're in.".to_string(),
        Command::GoInto(name) => format!("Goes into \"{name}\"."),
        Command::GoBack => "Goes up to the folder this one is in.".to_string(),
        Command::GoHome => "Goes to your home folder.".to_string(),
        Command::ShowFiles(folder) => format!("Lists the files {}.", here(folder)),
        Command::ShowAllFiles(folder) => {
            format!("Lists every file {}, hidden ones too.", here(folder))
        }
        Command::ShowDetails(folder) => {
            format!("Lists the files {} with sizes and dates.", here(folder))
        }
        Command::ShowTree(folder) => {
            format!(
                "Lists the files {}, and in every folder inside.",
                here(folder)
            )
        }
        Command::ShowSize(name) => format!("Shows how much space \"{name}\" takes."),
        Command::Read(name) => format!("Shows what's written in \"{name}\"."),
        Command::MakeFolder(name) => format!("Makes a new folder called \"{name}\"."),
        Command::MakeFile(name) => format!("Makes an empty file called \"{name}\"."),
        Command::Write { file, .. } => {
            format!("Puts your text into \"{file}\", replacing what was there.")
        }
        Command::Add { file, .. } => format!("Adds your text to the end of \"{file}\"."),
        Command::Open(name) => format!("Opens \"{name}\" in its usual app."),
        Command::Copy { from, to } => format!("Copies \"{from}\" to \"{to}\"."),
        Command::Move { from, to } => format!("Moves \"{from}\" to \"{to}\"."),
        Command::Rename { from, to } => format!("Renames \"{from}\" to \"{to}\"."),
        Command::Delete(name) => format!("Moves \"{name}\" to the Trash."),
        Command::FindFile(name) => format!("Looks for files with \"{name}\" in their name."),
        Command::FindText { text, place } => {
            format!("Looks for \"{text}\" inside \"{place}\".")
        }
        Command::Clear => "Clears the screen.".to_string(),
        Command::Tool(tool) => about_tool(tool),
        Command::Help(_) | Command::Version | Command::Bye => String::new(),
    }
}

fn about_tool(tool: &ToolCommand) -> String {
    match tool {
        ToolCommand::Install(name) => format!("Installs {name}."),
        ToolCommand::Uninstall(name) => format!("Removes {name}."),
        ToolCommand::UpdateAll => "Updates everything you installed.".to_string(),
        ToolCommand::ShowTools => "Shows tools you can install.".to_string(),
        ToolCommand::ShowInstalled => "Lists what you installed.".to_string(),
        ToolCommand::IsInstalled(name) => format!("Checks whether {name} is installed."),
        ToolCommand::SearchTools(text) => format!("Searches for tools matching \"{text}\"."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mac() -> Setup {
        Setup {
            shell: Shell::Unix,
            safe_delete: Some("trash".to_string()),
            has_homebrew: true,
        }
    }

    fn typed(plan: &Plan) -> &str {
        match &plan.then {
            Then::Type { command, .. } => command,
            other => panic!("expected something to type, got {other:?}"),
        }
    }

    /// A fresh folder for one test, deleted again when the test ends.
    struct Folder(std::path::PathBuf);

    impl Folder {
        fn new(test: &str) -> Folder {
            let path = std::env::temp_dir().join(format!("noob-bar-{test}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("make test folder");
            Folder(path)
        }

        fn file(&self, name: &str, text: &str) -> String {
            let path = self.0.join(name);
            std::fs::write(&path, text).expect("write test file");
            path.display().to_string()
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn plain_english_becomes_the_real_command_to_type() {
        let plan = plan("make folder photos", &mac());
        assert_eq!(plan.ask, None, "making a folder is safe");
        assert_eq!(typed(&plan), "mkdir -p photos");
    }

    #[test]
    fn the_plan_explains_what_the_command_does() {
        match plan("make folder photos", &mac()).then {
            Then::Type { about, .. } => assert!(about.contains("photos"), "{about}"),
            other => panic!("expected something to type, got {other:?}"),
        }
    }

    #[test]
    fn moving_around_works_because_the_shell_itself_moves() {
        assert_eq!(typed(&plan("go into photos", &mac())), "cd photos");
        assert_eq!(typed(&plan("go back", &mac())), "cd ..");
    }

    #[test]
    fn names_with_spaces_are_quoted_for_the_shell() {
        assert_eq!(
            typed(&plan("make folder \"my photos\"", &mac())),
            "mkdir -p 'my photos'"
        );
    }

    #[test]
    fn delete_uses_the_trash_command_and_asks_first() {
        let folder = Folder::new("delete-trash");
        let file = folder.file("old.txt", "bye");
        let plan = plan(&format!("delete {file}"), &mac());
        assert!(plan.ask.is_some(), "deleting asks first");
        let quoted = real::quote_name(&file, Shell::Unix);
        assert_eq!(typed(&plan), format!("trash {quoted}"));
    }

    #[test]
    fn delete_without_a_trash_command_is_done_by_noob_never_rm() {
        let folder = Folder::new("delete-here");
        let file = folder.file("old.txt", "bye");
        let setup = Setup {
            safe_delete: None,
            ..mac()
        };
        let plan = plan(&format!("delete {file}"), &setup);
        assert!(plan.ask.is_some());
        assert_eq!(plan.then, Then::RunHere(Command::Delete(file)));
    }

    #[test]
    fn deleting_something_missing_says_so_instead_of_asking() {
        let plan = plan("delete no-such-file-here.txt", &mac());
        assert_eq!(plan.ask, None);
        assert!(matches!(&plan.then, Then::Say(lines) if matches!(lines[..], [Line::Problem(_)])));
    }

    #[test]
    fn replacing_a_files_text_asks_first() {
        let folder = Folder::new("write");
        let file = folder.file("notes.txt", "keep me");
        let plan = plan(&format!("write hello into {file}"), &mac());
        assert!(plan.ask.is_some());
        let quoted = real::quote_name(&file, Shell::Unix);
        assert_eq!(typed(&plan), format!("echo hello > {quoted}"));
    }

    #[test]
    fn installing_asks_first_and_types_the_homebrew_command() {
        let plan = plan("install git", &mac());
        assert!(plan.ask.is_some(), "installing software asks first");
        assert!(typed(&plan).starts_with("brew install"), "{plan:?}");
    }

    #[test]
    fn installing_without_homebrew_suggests_getting_it_first() {
        let setup = Setup {
            has_homebrew: false,
            ..mac()
        };
        let plan = plan("install git", &setup);
        assert_eq!(plan.ask, None);
        assert!(
            matches!(&plan.then, Then::Say(lines) if matches!(&lines[..], [Line::Hint(hint)] if hint.contains("install homebrew"))),
            "{plan:?}"
        );
    }

    #[test]
    fn installing_homebrew_itself_does_not_need_homebrew() {
        let setup = Setup {
            has_homebrew: false,
            ..mac()
        };
        let plan = plan("install homebrew", &setup);
        assert!(plan.ask.is_some());
        assert!(typed(&plan).contains("curl"), "{plan:?}");
    }

    #[test]
    fn a_name_with_an_invisible_character_is_refused_not_changed() {
        // A tab inside a name can't be typed into a prompt safely; dropping it
        // would point the command at a different file.
        let plan = plan("make folder \"a\tb\"", &mac());
        assert!(
            matches!(&plan.then, Then::Say(lines) if matches!(lines[..], [Line::Problem(_)])),
            "{plan:?}"
        );
    }

    #[test]
    fn help_has_no_real_command_so_noob_answers_it() {
        let plan = plan("help", &mac());
        assert_eq!(plan.then, Then::RunHere(Command::Help(None)));
    }

    #[test]
    fn bye_explains_how_to_leave() {
        let plan = plan("bye", &mac());
        assert!(matches!(&plan.then, Then::Say(lines) if matches!(lines[..], [Line::Hint(_)])));
    }

    #[test]
    fn something_noob_does_not_understand_gets_a_hint() {
        let plan = plan("flibber the jabberwock", &mac());
        assert!(matches!(&plan.then, Then::Say(lines) if matches!(lines[..], [Line::Hint(_)])));
    }

    #[test]
    fn this_computer_is_looked_at_without_crashing() {
        let setup = Setup::detect();
        assert_eq!(setup.shell, Shell::current());
        if cfg!(windows) {
            assert_eq!(setup.safe_delete, None, "Noob uses the Recycle Bin itself");
        }
    }

    #[test]
    fn nothing_typed_does_nothing() {
        assert_eq!(plan("   ", &mac()).then, Then::Say(Vec::new()));
    }

    #[test]
    fn windows_gets_powershell_commands() {
        let setup = Setup {
            shell: Shell::PowerShell,
            safe_delete: None,
            has_homebrew: false,
        };
        let plan = plan("make folder photos", &setup);
        assert!(typed(&plan).contains("photos"));
        let install = super::plan("install git", &setup);
        assert!(typed(&install).starts_with("winget"), "{install:?}");
    }
}
