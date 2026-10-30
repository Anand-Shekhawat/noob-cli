//! Noob's own list of tools for beginners, with the exact name each installer knows
//! them by. Every Homebrew name and winget ID here was checked against the real
//! catalogs (October 2026).

/// Sections of `show tools`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Basics,
    Building,
    Apps,
    Helpers,
}

impl Group {
    pub const ALL: [Group; 4] = [Group::Basics, Group::Building, Group::Apps, Group::Helpers];

    pub fn title(self) -> &'static str {
        match self {
            Group::Basics => "Developer basics",
            Group::Building => "For building software",
            Group::Apps => "Everyday apps",
            Group::Helpers => "Small terminal helpers",
        }
    }
}

/// Where a tool comes from on a Mac (Linux uses the same, through Homebrew).
#[derive(Debug, PartialEq, Eq)]
pub enum MacSource {
    /// A Homebrew "formula": a program for the terminal.
    Formula(&'static str),
    /// A Homebrew "cask": an app with a window.
    Cask(&'static str),
    /// The tool's own official installer, run as one terminal command.
    Script(&'static str),
    /// Apple's Command Line Tools, which come from `xcode-select --install`.
    XcodeTools,
}

/// Where a tool comes from on Windows.
#[derive(Debug, PartialEq, Eq)]
pub enum WindowsSource {
    /// A winget package ID, plus any extra settings for its installer.
    Winget {
        id: &'static str,
        extra: &'static [&'static str],
    },
    /// Not for Windows (or Windows already has it). The sentence explains.
    NotAvailable(&'static str),
}

pub struct Tool {
    /// Ways to ask for it, all lowercase. The first one is shown in `show tools`.
    pub names: &'static [&'static str],
    pub title: &'static str,
    pub about: &'static str,
    pub group: Group,
    pub mac: MacSource,
    pub windows: WindowsSource,
    /// Programs that are there once it's installed (used by `is git installed`).
    pub programs: &'static [&'static str],
    /// For Mac apps: the app's name in the Applications folder, so Noob can tell
    /// when it's already there (even if it wasn't installed with Homebrew).
    pub mac_app: Option<&'static str>,
    /// A tip shown after installing, if there's something left to do.
    pub after: Option<&'static str>,
}

impl Tool {
    pub fn main_name(&self) -> &'static str {
        self.names.first().copied().unwrap_or_default()
    }
}

/// Homebrew's official installer, from brew.sh.
pub const HOMEBREW_SCRIPT: &str = r#"/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#;

/// Rust's official installer, from rustup.rs. `-y` accepts the standard choices,
/// because you already said yes in Noob.
pub const RUSTUP_SCRIPT: &str =
    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y";

const NEW_WINDOW: &str = "Open a new terminal window so it can be found.";

pub static TOOLS: &[Tool] = &[
    // Developer basics
    Tool {
        names: &["homebrew", "brew"],
        title: "Homebrew",
        about: "the free \"app store\" for the terminal that Noob uses on a Mac",
        group: Group::Basics,
        mac: MacSource::Script(HOMEBREW_SCRIPT),
        windows: WindowsSource::NotAvailable(
            "Homebrew is for Macs. On Windows, Noob uses winget, which comes with Windows.",
        ),
        programs: &["brew"],
        mac_app: None,
        after: None,
    },
    Tool {
        names: &["git"],
        title: "Git",
        about: "keeps track of changes to your code",
        group: Group::Basics,
        mac: MacSource::Formula("git"),
        windows: WindowsSource::Winget {
            id: "Git.Git",
            extra: &[],
        },
        programs: &["git"],
        mac_app: None,
        after: None,
    },
    Tool {
        names: &["vscode", "vs code", "visual studio code", "code"],
        title: "Visual Studio Code",
        about: "a free, popular code editor",
        group: Group::Basics,
        mac: MacSource::Cask("visual-studio-code"),
        windows: WindowsSource::Winget {
            id: "Microsoft.VisualStudioCode",
            extra: &[],
        },
        programs: &["code"],
        mac_app: Some("Visual Studio Code.app"),
        after: None,
    },
    Tool {
        names: &["node", "nodejs", "node.js", "npm"],
        title: "Node.js",
        about: "runs JavaScript outside the browser (comes with npm)",
        group: Group::Basics,
        mac: MacSource::Formula("node"),
        windows: WindowsSource::Winget {
            id: "OpenJS.NodeJS.LTS",
            extra: &[],
        },
        programs: &["node"],
        mac_app: None,
        after: None,
    },
    Tool {
        names: &["python", "python3"],
        title: "Python",
        about: "a friendly programming language",
        group: Group::Basics,
        mac: MacSource::Formula("python"),
        windows: WindowsSource::Winget {
            id: "Python.Python.3.13",
            extra: &[],
        },
        programs: &["python3", "python"],
        mac_app: None,
        after: None,
    },
    // For building software
    Tool {
        names: &["rust", "rustup", "cargo"],
        title: "Rust",
        about: "the Rust language and its tools, like cargo",
        group: Group::Building,
        mac: MacSource::Script(RUSTUP_SCRIPT),
        windows: WindowsSource::Winget {
            id: "Rustlang.Rustup",
            extra: &[],
        },
        programs: &["rustup", "cargo"],
        mac_app: None,
        after: Some(NEW_WINDOW),
    },
    Tool {
        names: &["build tools", "xcode tools", "command line tools"],
        title: "Build tools",
        about: "what Rust, Python and others need to build things",
        group: Group::Building,
        mac: MacSource::XcodeTools,
        windows: WindowsSource::Winget {
            id: "Microsoft.VisualStudio.2022.BuildTools",
            // The C++ part is the bit Rust needs; on its own the installer leaves it out.
            extra: &[
                "--override",
                "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended",
            ],
        },
        programs: &[],
        mac_app: None,
        after: None,
    },
    // Everyday apps
    Tool {
        names: &["chrome", "google chrome"],
        title: "Google Chrome",
        about: "a web browser",
        group: Group::Apps,
        mac: MacSource::Cask("google-chrome"),
        windows: WindowsSource::Winget {
            id: "Google.Chrome",
            extra: &[],
        },
        programs: &[],
        mac_app: Some("Google Chrome.app"),
        after: None,
    },
    Tool {
        names: &["firefox"],
        title: "Firefox",
        about: "a web browser that cares about privacy",
        group: Group::Apps,
        mac: MacSource::Cask("firefox"),
        windows: WindowsSource::Winget {
            id: "Mozilla.Firefox",
            extra: &[],
        },
        programs: &[],
        mac_app: Some("Firefox.app"),
        after: None,
    },
    Tool {
        names: &["vlc"],
        title: "VLC",
        about: "plays almost any video or music file",
        group: Group::Apps,
        mac: MacSource::Cask("vlc"),
        windows: WindowsSource::Winget {
            id: "VideoLAN.VLC",
            extra: &[],
        },
        programs: &[],
        mac_app: Some("VLC.app"),
        after: None,
    },
    Tool {
        names: &["zoom"],
        title: "Zoom",
        about: "video calls",
        group: Group::Apps,
        mac: MacSource::Cask("zoom"),
        windows: WindowsSource::Winget {
            id: "Zoom.Zoom",
            extra: &[],
        },
        programs: &[],
        mac_app: Some("zoom.us.app"),
        after: None,
    },
    // Small terminal helpers
    Tool {
        names: &["tree"],
        title: "tree",
        about: "draws folders inside folders",
        group: Group::Helpers,
        mac: MacSource::Formula("tree"),
        windows: WindowsSource::NotAvailable(
            "Windows already has tree: in PowerShell, type tree /f (or in Noob: show tree).",
        ),
        programs: &["tree"],
        mac_app: None,
        after: None,
    },
    Tool {
        names: &["wget"],
        title: "wget",
        about: "downloads files from the web",
        group: Group::Helpers,
        mac: MacSource::Formula("wget"),
        windows: WindowsSource::Winget {
            id: "JernejSimoncic.Wget",
            extra: &[],
        },
        programs: &["wget"],
        mac_app: None,
        after: None,
    },
    Tool {
        names: &["jq"],
        title: "jq",
        about: "reads and tidies JSON data",
        group: Group::Helpers,
        mac: MacSource::Formula("jq"),
        windows: WindowsSource::Winget {
            id: "jqlang.jq",
            extra: &[],
        },
        programs: &["jq"],
        mac_app: None,
        after: None,
    },
    Tool {
        names: &["htop"],
        title: "htop",
        about: "shows what's running and how busy your computer is",
        group: Group::Helpers,
        mac: MacSource::Formula("htop"),
        windows: WindowsSource::NotAvailable(
            "htop isn't made for Windows. Task Manager (Ctrl+Shift+Esc) shows the same things.",
        ),
        programs: &["htop"],
        mac_app: None,
        after: None,
    },
];

/// The curated tool called `name` ("vscode", "VS Code" and "code" all work).
pub fn find(name: &str) -> Option<&'static Tool> {
    let wanted = name.trim().to_lowercase();
    TOOLS
        .iter()
        .find(|tool| tool.names.contains(&wanted.as_str()) || tool.title.to_lowercase() == wanted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn every_name_is_lowercase_and_used_once() {
        let mut seen = HashSet::new();
        for name in TOOLS.iter().flat_map(|tool| tool.names.iter()) {
            assert_eq!(*name, name.to_lowercase(), "{name}");
            assert!(seen.insert(*name), "{name} is used twice");
        }
    }

    #[test]
    fn tools_missing_on_windows_say_what_to_use_instead() {
        // Every tool has a Mac source (the type makes sure of it), so check that the
        // Windows side explains itself whenever a tool isn't there.
        for tool in TOOLS {
            if let WindowsSource::NotAvailable(why) = tool.windows {
                assert!(why.len() > 20, "{} needs a real explanation", tool.title);
            }
        }
    }

    #[test]
    fn winget_ids_look_like_publisher_dot_name() {
        for tool in TOOLS {
            if let WindowsSource::Winget { id, .. } = tool.windows {
                assert!(id.contains('.') && !id.contains(' '), "{id}");
            }
        }
    }

    #[test]
    fn every_group_has_tools() {
        for group in Group::ALL {
            assert!(TOOLS.iter().any(|tool| tool.group == group), "{group:?}");
        }
    }

    #[test]
    fn finds_tools_by_any_name_ignoring_capitals() {
        assert_eq!(
            find("VS Code").map(|tool| tool.title),
            Some("Visual Studio Code")
        );
        assert_eq!(
            find("Visual Studio Code").map(|tool| tool.main_name()),
            Some("vscode")
        );
        assert!(find("banana").is_none());
    }
}
