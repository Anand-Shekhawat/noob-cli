//! Tests for installing tools. They use `FakeSystem`, which pretends to be a computer:
//! it answers questions from a script and writes down every installer it would have run.
//! So these tests never install anything for real.

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};

use super::system::{Answer, Os, System};
use super::*;

struct FakeSystem {
    os: Os,
    /// Programs the terminal can find, by name.
    programs: Vec<(&'static str, PathBuf)>,
    /// Files that exist.
    files: RefCell<Vec<PathBuf>>,
    /// Files that appear once an installer has run (like Homebrew itself).
    appear_after_run: Vec<PathBuf>,
    /// Scripted answers: if the quiet command contains the text, answer with (worked, output).
    answers: Vec<(&'static str, bool, &'static str)>,
    /// Whether installers that run on screen succeed.
    installers_work: bool,
    home: Option<PathBuf>,
    /// Everything that ran on screen, like "brew install git".
    ran: RefCell<Vec<String>>,
}

impl FakeSystem {
    fn mac() -> Self {
        FakeSystem {
            os: Os::Mac,
            programs: vec![("brew", PathBuf::from("/opt/homebrew/bin/brew"))],
            files: RefCell::new(Vec::new()),
            appear_after_run: Vec::new(),
            answers: Vec::new(),
            installers_work: true,
            home: None,
            ran: RefCell::new(Vec::new()),
        }
    }

    fn mac_without_homebrew() -> Self {
        FakeSystem {
            programs: Vec::new(),
            ..FakeSystem::mac()
        }
    }

    fn windows() -> Self {
        FakeSystem {
            os: Os::Windows,
            programs: vec![("winget", PathBuf::from("C:/winget.exe"))],
            ..FakeSystem::mac()
        }
    }

    fn answering(mut self, contains: &'static str, worked: bool, text: &'static str) -> Self {
        self.answers.push((contains, worked, text));
        self
    }

    fn ran(&self) -> Vec<String> {
        self.ran.borrow().clone()
    }
}

/// "brew install git", from a program path and its arguments.
fn as_typed(program: &Path, args: &[&str]) -> String {
    let name = program
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default();
    std::iter::once(name)
        .chain(args.iter().map(|arg| arg.to_string()))
        .collect::<Vec<_>>()
        .join(" ")
}

impl System for FakeSystem {
    fn os(&self) -> Os {
        self.os
    }

    fn find(&self, program: &str) -> Option<PathBuf> {
        self.programs
            .iter()
            .find(|(name, _)| *name == program)
            .map(|(_, path)| path.clone())
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.borrow().iter().any(|file| file == path)
    }

    fn home(&self) -> Option<PathBuf> {
        self.home.clone()
    }

    fn run(&self, program: &Path, args: &[&str]) -> io::Result<bool> {
        self.ran.borrow_mut().push(as_typed(program, args));
        self.files
            .borrow_mut()
            .extend(self.appear_after_run.iter().cloned());
        Ok(self.installers_work)
    }

    fn ask(&self, program: &Path, args: &[&str]) -> io::Result<Answer> {
        let typed = as_typed(program, args);
        let (worked, text) = self
            .answers
            .iter()
            .find(|(contains, _, _)| typed.contains(contains))
            .map_or((false, ""), |(_, worked, text)| (*worked, *text));
        Ok(Answer {
            worked,
            text: text.to_string(),
        })
    }
}

fn yes(_: &str) -> bool {
    true
}

fn no(_: &str) -> bool {
    false
}

fn install(
    name: &str,
    system: &FakeSystem,
    answer: fn(&str) -> bool,
) -> Result<Reply, ActionError> {
    run_on(&Command::Install(name.to_string()), system, &mut { answer })
}

fn text(reply: Result<Reply, ActionError>) -> String {
    match reply {
        Ok(Reply::Done(text) | Reply::Show(text)) => text,
        Err(error) => error.to_string(),
    }
}

#[test]
fn installs_a_curated_tool_with_homebrew_after_yes() {
    let mac = FakeSystem::mac();
    let reply = install("git", &mac, yes);
    assert_eq!(mac.ran(), vec!["brew install git"]);
    assert!(matches!(reply, Ok(Reply::Done(message)) if message.starts_with("Installed Git")));
}

#[test]
fn the_question_says_what_and_where_from() {
    let mac = FakeSystem::mac();
    let mut asked = String::new();
    run_on(
        &Command::Install("git".to_string()),
        &mac,
        &mut |question: &str| {
            asked = question.to_string();
            false
        },
    )
    .unwrap();
    assert!(
        asked.starts_with("Install Git (keeps track of changes to your code) with Homebrew?"),
        "{asked}"
    );
}

#[test]
fn answering_no_installs_nothing() {
    let mac = FakeSystem::mac();
    assert_eq!(
        text(install("git", &mac, no)),
        "Okay, nothing was installed."
    );
    assert!(mac.ran().is_empty());
}

#[test]
fn apps_with_windows_are_casks() {
    let mac = FakeSystem::mac();
    install("VS Code", &mac, yes).unwrap();
    assert_eq!(mac.ran(), vec!["brew install --cask visual-studio-code"]);
}

#[test]
fn something_already_installed_is_left_alone() {
    let mac = FakeSystem::mac().answering("list --formula git", true, "git");
    assert!(text(install("git", &mac, yes)).starts_with("Git is already installed."));
    assert!(mac.ran().is_empty());
}

#[test]
fn without_homebrew_it_explains_how_to_get_it() {
    let mac = FakeSystem::mac_without_homebrew();
    assert!(text(install("git", &mac, yes)).contains("type: install homebrew"));
    assert!(mac.ran().is_empty());
}

#[test]
fn homebrew_is_found_even_when_the_terminal_cannot_find_it() {
    let mac = FakeSystem::mac_without_homebrew();
    mac.files
        .borrow_mut()
        .push(PathBuf::from("/opt/homebrew/bin/brew"));
    install("git", &mac, yes).unwrap();
    assert_eq!(mac.ran(), vec!["brew install git"]);
}

#[test]
fn a_failed_installer_is_reported() {
    let mac = FakeSystem {
        installers_work: false,
        ..FakeSystem::mac()
    };
    assert!(text(install("git", &mac, yes)).contains("stopped with a problem"));
}

#[test]
fn windows_uses_the_exact_winget_id() {
    let windows = FakeSystem::windows();
    install("vscode", &windows, yes).unwrap();
    assert_eq!(
        windows.ran(),
        vec!["winget install --id Microsoft.VisualStudioCode --exact --source winget"]
    );
}

#[test]
fn windows_build_tools_include_the_part_rust_needs() {
    let windows = FakeSystem::windows();
    install("build tools", &windows, yes).unwrap();
    assert!(windows.ran()[0].contains("Microsoft.VisualStudio.Workload.VCTools"));
}

#[test]
fn says_what_to_use_when_a_tool_is_not_for_this_computer() {
    let windows = FakeSystem::windows();
    assert!(text(install("htop", &windows, yes)).contains("Task Manager"));
    assert!(text(install("homebrew", &windows, yes)).contains("winget"));
    assert!(windows.ran().is_empty());
}

#[test]
fn rust_on_a_mac_uses_its_official_installer_without_homebrew() {
    let mac = FakeSystem::mac_without_homebrew();
    let reply = install("rust", &mac, yes);
    assert_eq!(
        mac.ran(),
        vec![format!("bash -c {}", catalog::RUSTUP_SCRIPT)]
    );
    assert!(text(reply).contains("Open a new terminal window"));
}

#[test]
fn installs_anything_homebrew_knows_by_its_exact_name() {
    let mac = FakeSystem::mac().answering("info --formula ripgrep", true, "");
    install("ripgrep", &mac, yes).unwrap();
    assert_eq!(mac.ran(), vec!["brew install ripgrep"]);
}

#[test]
fn an_unknown_name_shows_what_homebrew_has_instead() {
    let mac =
        FakeSystem::mac().answering("search vide", true, "==> Formulae\nvidir\nvideo-compare\n");
    let shown = text(install("vide", &mac, yes));
    assert!(shown.contains("  video-compare"), "{shown}");
    assert!(shown.contains("like: install vidir"), "{shown}");
    assert!(mac.ran().is_empty());
}

#[test]
fn names_that_look_like_options_are_refused() {
    let mac = FakeSystem::mac();
    assert!(text(install("--force", &mac, yes)).contains("doesn't look like a tool name"));
    assert!(mac.ran().is_empty());
}

#[test]
fn installing_homebrew_runs_the_official_script_and_offers_to_set_up_the_terminal() {
    let home = std::env::temp_dir().join(format!("noob-brew-home-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let mac = FakeSystem {
        appear_after_run: vec![PathBuf::from("/opt/homebrew/bin/brew")],
        home: Some(home.clone()),
        ..FakeSystem::mac_without_homebrew()
    };

    let reply = text(install("homebrew", &mac, yes));
    assert_eq!(
        mac.ran(),
        vec![format!("bash -c {}", catalog::HOMEBREW_SCRIPT)]
    );
    let settings = std::fs::read_to_string(home.join(".zprofile")).unwrap();
    assert!(
        settings.contains(r#"eval "$(/opt/homebrew/bin/brew shellenv)""#),
        "{settings}"
    );
    assert!(reply.contains("Open a new terminal window"), "{reply}");

    // Asking again doesn't add the line twice.
    setup::after_homebrew(&mac, &mut yes).unwrap();
    let again = std::fs::read_to_string(home.join(".zprofile")).unwrap();
    assert_eq!(again.matches("shellenv").count(), 1);
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn homebrew_is_not_installed_twice() {
    let mac = FakeSystem::mac();
    assert!(text(install("homebrew", &mac, yes)).contains("already installed"));
    assert!(mac.ran().is_empty());
}

#[test]
fn uninstalls_only_what_is_installed() {
    let mac = FakeSystem::mac().answering("list --cask vlc", true, "vlc");
    run_on(&Command::Uninstall("vlc".to_string()), &mac, &mut yes).unwrap();
    assert_eq!(mac.ran(), vec!["brew uninstall --cask vlc"]);

    let empty = FakeSystem::mac();
    let shown = text(run_on(
        &Command::Uninstall("vlc".to_string()),
        &empty,
        &mut yes,
    ));
    assert!(shown.contains("nothing to remove"), "{shown}");
    assert!(empty.ran().is_empty());
}

#[test]
fn update_everything_fetches_then_upgrades() {
    let mac = FakeSystem::mac();
    run_on(&Command::UpdateAll, &mac, &mut yes).unwrap();
    assert_eq!(mac.ran(), vec!["brew update", "brew upgrade"]);

    let windows = FakeSystem::windows();
    run_on(&Command::UpdateAll, &windows, &mut yes).unwrap();
    assert_eq!(windows.ran(), vec!["winget upgrade --all --source winget"]);
}

#[test]
fn is_installed_says_how_and_where() {
    let mac = FakeSystem {
        programs: vec![
            ("brew", PathBuf::from("/opt/homebrew/bin/brew")),
            ("git", PathBuf::from("/opt/homebrew/bin/git")),
        ],
        ..FakeSystem::mac()
    }
    .answering("list --formula git", true, "git");
    let yes_answer = text(run_on(
        &Command::IsInstalled("git".to_string()),
        &mac,
        &mut yes,
    ));
    assert_eq!(
        yes_answer,
        "Yes, Git is installed (with Homebrew), at /opt/homebrew/bin/git."
    );

    let no_answer = text(run_on(
        &Command::IsInstalled("jq".to_string()),
        &mac,
        &mut yes,
    ));
    assert_eq!(
        no_answer,
        "No, jq isn't installed. To install it, type: install jq"
    );
}

#[test]
fn show_tools_lists_every_group_and_marks_what_this_computer_cannot_have() {
    let shown = text(run_on(
        &Command::ShowTools,
        &FakeSystem::windows(),
        &mut yes,
    ));
    for group in catalog::Group::ALL {
        assert!(shown.contains(group.title()));
    }
    assert!(
        shown
            .lines()
            .any(|line| line.contains("htop") && line.contains("not on this computer"))
    );
}

#[test]
fn reads_winget_search_tables() {
    // Like real winget: a spinner drawn with "\r", then columns sized to fit.
    let table = concat!(
        "\r   - \r  \\ \r",
        "Name               Id                         Version Source\n",
        "-------------------------------------------------------------\n",
        "Visual Studio Code Microsoft.VisualStudioCode 1.95.0  winget\n",
        "VLC media player   VideoLAN.VLC               3.0.21  winget\n",
    );
    assert_eq!(
        package::winget_ids(table),
        vec!["Microsoft.VisualStudioCode", "VideoLAN.VLC"]
    );
    assert!(package::winget_ids("No package found matching input criteria.").is_empty());
}

#[test]
fn real_commands_match_the_computer() {
    let install = Command::Install("vscode".to_string());
    assert_eq!(
        real_command(&install, Os::Mac).unwrap(),
        "brew install --cask visual-studio-code"
    );
    assert_eq!(
        real_command(&install, Os::Windows).unwrap(),
        "winget install --id Microsoft.VisualStudioCode --exact --source winget"
    );
    assert_eq!(
        real_command(&Command::UpdateAll, Os::Mac).unwrap(),
        "brew update && brew upgrade"
    );
}
