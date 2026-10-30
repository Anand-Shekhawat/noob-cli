//! Tests for installing tools. They use `FakeSystem`, which pretends to be a computer:
//! it answers questions from a script and writes down every installer it would have run.
//! So these tests never install anything for real.

use std::cell::RefCell;
use std::io;
use std::path::{Path, PathBuf};

use super::system::{Answer, Os, System};
use super::*;
use crate::command::ToolCommand;

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
    run_on(&ToolCommand::Install(name.to_string()), system, &mut {
        answer
    })
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
        &ToolCommand::Install("git".to_string()),
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
    // rustup puts its programs in ~/.cargo/bin, which a new terminal window finds.
    let mac = FakeSystem {
        home: Some(PathBuf::from("/Users/sam")),
        appear_after_run: vec![PathBuf::from("/Users/sam/.cargo/bin/rustup")],
        ..FakeSystem::mac_without_homebrew()
    };
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
    setup::after_homebrew(&mac, &mut yes);
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
    run_on(&ToolCommand::Uninstall("vlc".to_string()), &mac, &mut yes).unwrap();
    assert_eq!(mac.ran(), vec!["brew uninstall --cask vlc"]);

    let empty = FakeSystem::mac();
    let shown = text(run_on(
        &ToolCommand::Uninstall("vlc".to_string()),
        &empty,
        &mut yes,
    ));
    assert!(shown.contains("nothing to remove"), "{shown}");
    assert!(empty.ran().is_empty());
}

#[test]
fn update_everything_fetches_then_upgrades() {
    let mac = FakeSystem::mac();
    run_on(&ToolCommand::UpdateAll, &mac, &mut yes).unwrap();
    assert_eq!(mac.ran(), vec!["brew update", "brew upgrade"]);

    let windows = FakeSystem::windows();
    run_on(&ToolCommand::UpdateAll, &windows, &mut yes).unwrap();
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
        &ToolCommand::IsInstalled("git".to_string()),
        &mac,
        &mut yes,
    ));
    assert_eq!(
        yes_answer,
        "Yes, Git is installed (with Homebrew), at /opt/homebrew/bin/git."
    );

    let no_answer = text(run_on(
        &ToolCommand::IsInstalled("jq".to_string()),
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
        &ToolCommand::ShowTools,
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
    let install = ToolCommand::Install("vscode".to_string());
    assert_eq!(
        real_command(&install, Os::Mac).unwrap(),
        "brew install --cask visual-studio-code"
    );
    assert_eq!(
        real_command(&install, Os::Windows).unwrap(),
        "winget install --id Microsoft.VisualStudioCode --exact --source winget"
    );
    assert_eq!(
        real_command(&ToolCommand::UpdateAll, Os::Mac).unwrap(),
        "brew update && brew upgrade"
    );
}

#[test]
fn names_with_a_slash_are_refused_because_they_add_someone_elses_source() {
    let mac = FakeSystem::mac().answering("info --formula", true, "");
    assert!(text(install("someone/tap/tool", &mac, yes)).contains("doesn't look like a tool name"));
    assert!(text(install(".hidden", &mac, yes)).contains("doesn't look like a tool name"));
    assert!(mac.ran().is_empty());
}

#[test]
fn an_app_already_in_applications_counts_as_installed() {
    let mac = FakeSystem::mac();
    // Built the same way the code builds it, so this test also passes on Windows.
    let app = Path::new("/Applications").join("VLC.app");
    mac.files.borrow_mut().push(app.clone());
    let shown = text(install("vlc", &mac, yes));
    assert_eq!(
        shown,
        format!("VLC is already on your Mac, at {}.", app.display())
    );
    assert!(mac.ran().is_empty());
}

#[test]
fn an_installer_script_that_installed_nothing_is_not_called_a_success() {
    // With no internet, Homebrew's script downloads nothing and still "succeeds".
    let mac = FakeSystem::mac_without_homebrew();
    let shown = text(install("homebrew", &mac, yes));
    assert!(
        shown.contains("I can't find Homebrew afterwards"),
        "{shown}"
    );
}

#[test]
fn placeholder_programs_do_not_count_as_installed() {
    // Before Apple's tools are installed, /usr/bin/git only offers to install them.
    let mac = FakeSystem {
        programs: vec![
            ("brew", PathBuf::from("/opt/homebrew/bin/brew")),
            ("git", PathBuf::from("/usr/bin/git")),
        ],
        ..FakeSystem::mac()
    };
    let shown = text(run_on(
        &ToolCommand::IsInstalled("git".to_string()),
        &mac,
        &mut yes,
    ));
    assert!(shown.starts_with("No, Git isn't installed."), "{shown}");

    // Windows' python in WindowsApps only opens the Store.
    let windows = FakeSystem {
        programs: vec![
            ("winget", PathBuf::from("C:/winget.exe")),
            (
                "python",
                PathBuf::from("C:/Users/sam/AppData/Local/Microsoft/WindowsApps/python.exe"),
            ),
        ],
        ..FakeSystem::windows()
    };
    let shown = text(run_on(
        &ToolCommand::IsInstalled("python".to_string()),
        &windows,
        &mut yes,
    ));
    assert!(shown.starts_with("No, Python isn't installed."), "{shown}");
}

#[test]
fn saying_no_to_the_settings_change_explains_how_to_do_it_later() {
    let home = std::env::temp_dir().join(format!("noob-brew-no-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&home);
    std::fs::create_dir_all(&home).unwrap();
    let mac = FakeSystem {
        home: Some(home.clone()),
        ..FakeSystem::mac_without_homebrew()
    };
    mac.files
        .borrow_mut()
        .push(PathBuf::from("/opt/homebrew/bin/brew"));

    let advice = setup::after_homebrew(&mac, &mut no);
    assert!(advice.starts_with("To set it up yourself"), "{advice}");
    assert!(!home.join(".zprofile").exists(), "nothing was changed");
    let _ = std::fs::remove_dir_all(&home);
}

#[test]
fn windows_uninstalls_with_winget() {
    let windows = FakeSystem::windows().answering("list --id VideoLAN.VLC", true, "VLC");
    run_on(
        &ToolCommand::Uninstall("vlc".to_string()),
        &windows,
        &mut yes,
    )
    .unwrap();
    assert_eq!(
        windows.ran(),
        vec!["winget uninstall --id VideoLAN.VLC --exact --source winget"]
    );
}

#[test]
fn mac_apps_are_not_offered_on_linux() {
    let linux = FakeSystem {
        os: Os::Linux,
        ..FakeSystem::mac()
    };
    assert!(text(install("chrome", &linux, yes)).contains("Mac app"));
    assert!(linux.ran().is_empty());
}

#[test]
fn search_still_shows_noobs_own_list_without_homebrew() {
    let mac = FakeSystem::mac_without_homebrew();
    let shown = text(run_on(
        &ToolCommand::SearchTools("video".to_string()),
        &mac,
        &mut yes,
    ));
    assert!(shown.contains("vlc (VLC)"), "{shown}");
    assert!(shown.contains("install homebrew"), "{shown}");
}

#[test]
fn winget_table_footers_are_not_mistaken_for_ids() {
    let table = concat!(
        "Name  Id             Version Source\n",
        "-----------------------------------\n",
        "Jq    jqlang.jq      1.7.1   winget\n",
        "2 upgrades available.\n",
    );
    assert_eq!(package::winget_ids(table), vec!["jqlang.jq"]);
}

#[test]
fn the_real_command_for_checking_an_app_uses_the_installer() {
    let check = ToolCommand::IsInstalled("chrome".to_string());
    assert_eq!(
        real_command(&check, Os::Mac).unwrap(),
        "brew list --cask google-chrome"
    );
    let program = ToolCommand::IsInstalled("git".to_string());
    assert_eq!(
        real_command(&program, Os::Windows).unwrap(),
        "Get-Command git"
    );
}
