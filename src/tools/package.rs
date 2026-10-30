//! Turning "install git" into the exact steps Homebrew or winget needs.

use std::path::{Path, PathBuf};

use super::catalog::{MacSource, Tool, WindowsSource};
use super::system::{Os, System};
use crate::actions::ActionError;

/// Where Homebrew puts itself: Apple-chip Macs, Intel Macs, and Linux.
const HOMEBREW_PLACES: [&str; 3] = [
    "/opt/homebrew/bin/brew",
    "/usr/local/bin/brew",
    "/home/linuxbrew/.linuxbrew/bin/brew",
];

/// The installer this computer uses.
pub enum Manager {
    Homebrew(PathBuf),
    Winget(PathBuf),
}

impl Manager {
    pub fn name(&self) -> &'static str {
        match self {
            Manager::Homebrew(_) => "Homebrew",
            Manager::Winget(_) => "winget",
        }
    }

    pub fn program(&self) -> &Path {
        match self {
            Manager::Homebrew(path) | Manager::Winget(path) => path,
        }
    }
}

/// Homebrew, even if the terminal can't find it yet (a common hiccup right after
/// installing). Homebrew's own fixed places are checked first, so a program that just
/// happens to be called "brew" somewhere in PATH can't pretend to be it.
pub fn homebrew_location(system: &dyn System) -> Option<PathBuf> {
    HOMEBREW_PLACES
        .iter()
        .map(PathBuf::from)
        .find(|place| system.exists(place))
        .or_else(|| system.find("brew"))
}

/// True for the places Homebrew installs itself.
pub fn is_standard_homebrew(path: &Path) -> bool {
    HOMEBREW_PLACES.iter().any(|place| Path::new(place) == path)
}

pub fn manager(system: &dyn System) -> Result<Manager, ActionError> {
    match system.os() {
        Os::Mac | Os::Linux => homebrew_location(system)
            .map(Manager::Homebrew)
            .ok_or_else(|| {
                ActionError::Explained(
                    "Homebrew isn't installed yet. It's what Noob uses to install tools. \
                 To get it, type: install homebrew"
                        .to_string(),
                )
            }),
        Os::Windows => system.find("winget").map(Manager::Winget).ok_or_else(|| {
            ActionError::Explained(
                "winget isn't available. It comes with Windows 10 and 11 as \"App Installer\": \
                 open the Microsoft Store, search for App Installer, and install or update it."
                    .to_string(),
            )
        }),
    }
}

/// Exactly what to install.
#[derive(Debug, PartialEq, Eq)]
pub enum Package {
    Formula(String),
    Cask(String),
    Script(&'static str),
    XcodeTools,
    Winget { id: String, extra: Vec<String> },
}

impl Package {
    /// The package for a curated tool on this kind of computer, or why there isn't one.
    pub fn for_tool(tool: &Tool, os: Os) -> Result<Package, &'static str> {
        match os {
            Os::Mac | Os::Linux => match tool.mac {
                MacSource::Formula(name) => Ok(Package::Formula(name.to_string())),
                MacSource::Cask(name) if os == Os::Mac => Ok(Package::Cask(name.to_string())),
                MacSource::Cask(_) => Err("That's a Mac app, so Noob can't install it on Linux."),
                MacSource::Script(script) => Ok(Package::Script(script)),
                MacSource::XcodeTools if os == Os::Mac => Ok(Package::XcodeTools),
                MacSource::XcodeTools => {
                    Err("On Linux, build tools come with your system's own installer.")
                }
            },
            Os::Windows => match tool.windows {
                WindowsSource::Winget { id, extra } => Ok(Package::Winget {
                    id: id.to_string(),
                    extra: extra.iter().map(|part| part.to_string()).collect(),
                }),
                WindowsSource::NotAvailable(why) => Err(why),
            },
        }
    }

    /// Does this need Homebrew or winget? (Official scripts and Apple's tools don't.)
    pub fn needs_manager(&self) -> bool {
        !matches!(self, Package::Script(_) | Package::XcodeTools)
    }

    pub fn install_args(&self) -> Vec<String> {
        match self {
            Package::Formula(name) => words(&["install", name]),
            Package::Cask(name) => words(&["install", "--cask", name]),
            Package::Script(script) => words(&["-c", script]),
            Package::XcodeTools => words(&["--install"]),
            Package::Winget { id, extra } => {
                let mut args = words(&["install", "--id", id, "--exact", "--source", "winget"]);
                args.extend(extra.iter().cloned());
                args
            }
        }
    }

    /// `None` for things Noob can't take away (official scripts, Apple's tools).
    pub fn uninstall_args(&self) -> Option<Vec<String>> {
        match self {
            Package::Formula(name) => Some(words(&["uninstall", name])),
            Package::Cask(name) => Some(words(&["uninstall", "--cask", name])),
            Package::Winget { id, .. } => Some(words(&[
                "uninstall",
                "--id",
                id,
                "--exact",
                "--source",
                "winget",
            ])),
            Package::Script(_) | Package::XcodeTools => None,
        }
    }

    /// The quiet question that tells us whether it's installed already.
    pub fn check_args(&self) -> Option<Vec<String>> {
        match self {
            Package::Formula(name) => Some(words(&["list", "--formula", name])),
            Package::Cask(name) => Some(words(&["list", "--cask", name])),
            Package::Winget { id, .. } => Some(words(&[
                "list",
                "--id",
                id,
                "--exact",
                "--source",
                "winget",
                "--disable-interactivity",
            ])),
            Package::Script(_) | Package::XcodeTools => None,
        }
    }

    /// The command as you'd type it yourself.
    pub fn real_install(&self) -> String {
        match self {
            Package::Script(script) => script.to_string(),
            Package::XcodeTools => "xcode-select --install".to_string(),
            Package::Winget { .. } => format!("winget {}", shown(&self.install_args())),
            Package::Formula(_) | Package::Cask(_) => {
                format!("brew {}", shown(&self.install_args()))
            }
        }
    }

    pub fn real_uninstall(&self) -> Option<String> {
        let args = self.uninstall_args()?;
        let program = if matches!(self, Package::Winget { .. }) {
            "winget"
        } else {
            "brew"
        };
        Some(format!("{program} {}", shown(&args)))
    }
}

fn words(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|part| part.to_string()).collect()
}

/// Arguments as you'd type them: anything with a space goes in quotes.
fn shown(args: &[String]) -> String {
    args.iter()
        .map(|arg| {
            if arg.contains(' ') {
                format!("\"{arg}\"")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Tool names are letters, numbers and `. _ + @ -`. They never start with "-" (an
/// installer would read that as an option) or "." and never contain "/": in Homebrew,
/// "someone/tap/tool" quietly adds someone else's source of programs, which isn't the
/// official catalog Noob promises.
pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['-', '.'])
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._+@-".contains(c))
}

/// Search text can have spaces ("video player"), but not control characters, and
/// mustn't look like an option.
pub fn valid_search(text: &str) -> bool {
    let text = text.trim();
    !text.is_empty()
        && text.len() <= 100
        && !text.starts_with('-')
        && !text.chars().any(char::is_control)
}

/// Asks the installer whether it knows a package called exactly `name`.
pub fn exact_package(name: &str, manager: &Manager, system: &dyn System) -> Option<Package> {
    let knows = |args: &[&str]| {
        system
            .ask(manager.program(), args)
            .is_ok_and(|answer| answer.worked)
    };
    match manager {
        Manager::Homebrew(_) if knows(&["info", "--formula", name]) => {
            Some(Package::Formula(name.to_string()))
        }
        Manager::Homebrew(_) if knows(&["info", "--cask", name]) => {
            Some(Package::Cask(name.to_string()))
        }
        Manager::Winget(_)
            if knows(&[
                "show",
                "--id",
                name,
                "--exact",
                "--source",
                "winget",
                "--disable-interactivity",
            ]) =>
        {
            Some(Package::Winget {
                id: name.to_string(),
                extra: Vec::new(),
            })
        }
        _ => None,
    }
}

/// Searches the installer's catalog. Returns names (Homebrew) or IDs (winget).
pub fn search(
    text: &str,
    manager: &Manager,
    system: &dyn System,
) -> Result<Vec<String>, ActionError> {
    let args: &[&str] = match manager {
        Manager::Homebrew(_) => &["search", text],
        Manager::Winget(_) => &[
            "search",
            text,
            "--source",
            "winget",
            "--disable-interactivity",
        ],
    };
    let answer = system.ask(manager.program(), args).map_err(|error| {
        ActionError::Explained(format!("I couldn't ask {}: {error}", manager.name()))
    })?;
    Ok(match manager {
        Manager::Homebrew(_) => brew_names(&answer.text),
        Manager::Winget(_) => winget_ids(&answer.text),
    })
}

/// `brew search` prints names, with "==> Formulae" and "==> Casks" headings.
pub fn brew_names(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.starts_with("==>"))
        .flat_map(str::split_whitespace)
        .filter(|name| !name.chars().any(char::is_control))
        .map(str::to_string)
        .collect()
}

/// `winget search` prints a table. The IDs are in the column under the heading "Id".
pub fn winget_ids(text: &str) -> Vec<String> {
    // winget redraws a spinner with "\r"; only the last drawing on each line counts.
    let lines: Vec<&str> = text
        .lines()
        .map(|line| line.rsplit('\r').next().unwrap_or(line))
        .collect();
    let Some(header_at) = lines
        .iter()
        .position(|line| line.contains(" Id ") && line.contains("Version"))
    else {
        return Vec::new();
    };
    let header: Vec<char> = lines[header_at].chars().collect();
    let column = |title: &str| {
        let title: Vec<char> = title.chars().collect();
        header
            .windows(title.len())
            .position(|window| window == title.as_slice())
    };
    let (Some(id_start), Some(version_start)) =
        (column(" Id ").map(|at| at + 1), column("Version"))
    else {
        return Vec::new();
    };
    let Some(width) = version_start.checked_sub(id_start) else {
        return Vec::new();
    };
    lines
        .iter()
        .skip(header_at + 1)
        .filter(|line| !line.trim_start().starts_with('-'))
        .map(|line| {
            let id: String = line.chars().skip(id_start).take(width).collect();
            id.trim().to_string()
        })
        // Real winget IDs look like "Publisher.Name": this drops footer lines and junk.
        .filter(|id| {
            id.contains('.')
                && !id.contains(char::is_whitespace)
                && !id.chars().any(char::is_control)
        })
        .collect()
}
