//! Installing tools and apps: Homebrew on a Mac (and Linux), winget on Windows.
//!
//! Safety rules: Noob always says what it will install and where from, and asks first.
//! Installers run on screen, so if one asks for your password, you type it straight
//! to macOS or Windows. Noob never sees it.

mod catalog;
mod package;
mod setup;
pub mod system;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use catalog::{Group, TOOLS, Tool};
use package::{Manager, Package};
use system::{Os, RealSystem, System};

use crate::actions::{ActionError, Reply};
use crate::command::ToolCommand;

/// How Noob asks "are you sure?". The runner passes in a real question on screen;
/// tests pass in a pretend answer.
pub type Ask<'a> = &'a mut dyn FnMut(&str) -> bool;

/// Most search results shown at once.
const MAX_RESULTS: usize = 20;
/// Where Mac apps live.
const APPLICATIONS: &str = "/Applications";

pub fn run(command: &ToolCommand, ask: Ask) -> Result<Reply, ActionError> {
    run_on(command, &RealSystem, ask)
}

fn run_on(command: &ToolCommand, system: &dyn System, ask: Ask) -> Result<Reply, ActionError> {
    match command {
        ToolCommand::Install(name) => install(name, system, ask),
        ToolCommand::Uninstall(name) => uninstall(name, system, ask),
        ToolCommand::UpdateAll => update_all(system, ask),
        ToolCommand::ShowTools => Ok(show_tools(system.os())),
        ToolCommand::ShowInstalled => show_installed(system),
        ToolCommand::IsInstalled(name) => is_installed(name, system),
        ToolCommand::SearchTools(text) => search_tools(text, system),
    }
}

/// What to install: one of Noob's own tools, or something else Homebrew/winget has.
struct Choice {
    title: String,
    about: String,
    package: Package,
    tool: Option<&'static Tool>,
}

impl Choice {
    fn is_homebrew(&self) -> bool {
        self.tool.is_some_and(|tool| tool.main_name() == "homebrew")
    }
}

fn install(name: &str, system: &dyn System, ask: Ask) -> Result<Reply, ActionError> {
    let Some(choice) = choose(name, system)? else {
        return suggestions(name, system);
    };
    let manager = choice
        .package
        .needs_manager()
        .then(|| package::manager(system))
        .transpose()?;
    if let Some(already) = already_installed(&choice, manager.as_ref(), system) {
        return Ok(Reply::Show(already));
    }
    let question = format!(
        "Install {} ({}) {}? It can take a few minutes.",
        choice.title,
        choice.about,
        from(&choice.package)
    );
    if !ask(&question) {
        return Ok(Reply::Show("Okay, nothing was installed.".to_string()));
    }
    run_installer(
        &choice,
        manager.as_ref(),
        system,
        &choice.package.install_args(),
    )?;

    // Official scripts can finish "successfully" without installing anything (with no
    // internet, the download is just empty), so check before saying it worked.
    if matches!(choice.package, Package::Script(_))
        && already_installed(&choice, None, system).is_none()
    {
        return Err(ActionError::Explained(format!(
            "The installer finished, but I can't find {} afterwards. Check your internet \
             connection and the messages above, then try again.",
            choice.title
        )));
    }
    let mut message = vec![if choice.package == Package::XcodeTools {
        "A window should open: click Install there, then wait for it to finish.".to_string()
    } else {
        format!("Installed {}.", choice.title)
    }];
    message.extend(choice.tool.and_then(|tool| tool.after).map(str::to_string));
    if choice.is_homebrew() {
        message.push(setup::after_homebrew(system, ask));
    }
    Ok(Reply::Done(message.join(" ").trim().to_string()))
}

/// Works out what `name` means. `Ok(None)` means "no exact match: show suggestions".
fn choose(name: &str, system: &dyn System) -> Result<Option<Choice>, ActionError> {
    if let Some(tool) = catalog::find(name) {
        let package = Package::for_tool(tool, system.os())
            .map_err(|why| ActionError::Explained(why.to_string()))?;
        return Ok(Some(Choice {
            title: tool.title.to_string(),
            about: tool.about.to_string(),
            package,
            tool: Some(tool),
        }));
    }
    check_name(name)?;
    let manager = package::manager(system)?;
    Ok(
        package::exact_package(name, &manager, system).map(|package| Choice {
            title: name.to_string(),
            about: format!("from {}'s own catalog", manager.name()),
            package,
            tool: None,
        }),
    )
}

fn check_name(name: &str) -> Result<(), ActionError> {
    if package::valid_name(name) {
        Ok(())
    } else {
        Err(ActionError::Explained(format!(
            "\"{name}\" doesn't look like a tool name. To see what's available, type: show tools"
        )))
    }
}

/// "with Homebrew", or "with its official installer".
fn from(package: &Package) -> &'static str {
    match package {
        Package::Formula(_) | Package::Cask(_) => "with Homebrew",
        Package::Winget { .. } => "with winget",
        Package::Script(_) => "with its official installer",
        Package::XcodeTools => "from Apple",
    }
}

/// `Some(message)` if it's installed already, saying how.
fn already_installed(
    choice: &Choice,
    manager: Option<&Manager>,
    system: &dyn System,
) -> Option<String> {
    let title = &choice.title;
    let newest = "To get the newest version, type: update everything";
    let found = match &choice.package {
        Package::XcodeTools => xcode_tools_installed(system),
        Package::Script(_) if choice.is_homebrew() => package::homebrew_location(system).is_some(),
        Package::Script(_) => choice.tool.is_some_and(|tool| {
            tool.programs
                .iter()
                .any(|program| find_program(program, system).is_some())
        }),
        package => {
            if manager.is_some_and(|manager| installed_with(package, manager, system)) {
                return Some(format!("{title} is already installed. {newest}"));
            }
            // An app copied into Applications by hand isn't Homebrew's, but it's there.
            if let Some(app) = mac_app_path(choice.tool, system) {
                return Some(format!(
                    "{title} is already on your Mac, at {}.",
                    app.display()
                ));
            }
            false
        }
    };
    found.then(|| format!("{title} is already installed."))
}

fn xcode_tools_installed(system: &dyn System) -> bool {
    system
        .ask(Path::new("/usr/bin/xcode-select"), &["-p"])
        .is_ok_and(|answer| answer.worked)
}

fn mac_app_path(tool: Option<&Tool>, system: &dyn System) -> Option<PathBuf> {
    let app = Path::new(APPLICATIONS).join(tool?.mac_app?);
    (system.os() == Os::Mac && system.exists(&app)).then_some(app)
}

/// A program that's really there: on PATH, or where rustup puts things (~/.cargo/bin),
/// but not one of the placeholders computers come with.
fn find_program(program: &str, system: &dyn System) -> Option<PathBuf> {
    let in_cargo = system
        .home()
        .map(|home| home.join(".cargo").join("bin").join(program))
        .filter(|path| system.exists(path));
    system
        .find(program)
        .filter(|path| !is_placeholder(path, system))
        .or(in_cargo)
}

/// Macs come with stand-in `git` and `python3` in /usr/bin that only offer to install
/// Apple's tools, and Windows has a stand-in `python` that only opens the Store.
fn is_placeholder(path: &Path, system: &dyn System) -> bool {
    match system.os() {
        Os::Mac => path.starts_with("/usr/bin") && !xcode_tools_installed(system),
        Os::Windows => path
            .to_string_lossy()
            .to_lowercase()
            .contains("windowsapps"),
        Os::Linux => false,
    }
}

fn installed_with(package: &Package, manager: &Manager, system: &dyn System) -> bool {
    let Some(args) = package.check_args() else {
        return false;
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    system
        .ask(manager.program(), &args)
        .is_ok_and(|answer| answer.worked)
}

/// Runs the installer (or uninstaller) on screen.
fn run_installer(
    choice: &Choice,
    manager: Option<&Manager>,
    system: &dyn System,
    args: &[String],
) -> Result<(), ActionError> {
    let program = match (&choice.package, manager) {
        (Package::Script(_), _) => PathBuf::from("/bin/bash"),
        (Package::XcodeTools, _) => PathBuf::from("/usr/bin/xcode-select"),
        (_, Some(manager)) => manager.program().to_path_buf(),
        (_, None) => {
            return Err(ActionError::Explained(
                "I couldn't find the installer.".to_string(),
            ));
        }
    };
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    match system.run(&program, &args) {
        Ok(true) => Ok(()),
        Ok(false) => Err(ActionError::Explained(format!(
            "The installer stopped with a problem, so {} may not have changed. Its messages above say why.",
            choice.title
        ))),
        Err(error) => Err(ActionError::Explained(format!(
            "I couldn't start {}: {error}",
            program.display()
        ))),
    }
}

/// When `install` doesn't find an exact match, show what the installer does have.
fn suggestions(name: &str, system: &dyn System) -> Result<Reply, ActionError> {
    let manager = package::manager(system)?;
    let found = package::search(name, &manager, system)?;
    let Some(first) = found.first() else {
        return Ok(Reply::Show(format!(
            "I couldn't find \"{name}\" in {}. Check the spelling, or type: show tools",
            manager.name()
        )));
    };
    Ok(Reply::Show(format!(
        "I don't know a tool called \"{name}\". {} has these:\n{}\nTo install one, type install and its name, like: install {first}",
        manager.name(),
        listed(&found)
    )))
}

fn listed(names: &[String]) -> String {
    let mut lines: Vec<String> = names
        .iter()
        .take(MAX_RESULTS)
        .map(|name| format!("  {name}"))
        .collect();
    if names.len() > MAX_RESULTS {
        lines.push(format!("  … and {} more", names.len() - MAX_RESULTS));
    }
    lines.join("\n")
}

fn uninstall(name: &str, system: &dyn System, ask: Ask) -> Result<Reply, ActionError> {
    let Some(choice) = choose(name, system)? else {
        return Ok(Reply::Show(format!(
            "\"{name}\" isn't something {} has installed.",
            installer_name(system)
        )));
    };
    let Some(args) = choice.package.uninstall_args() else {
        return Err(ActionError::Explained(format!(
            "Noob can only uninstall things that came from Homebrew or winget, and {} didn't.",
            choice.title
        )));
    };
    let manager = package::manager(system)?;
    if !installed_with(&choice.package, &manager, system) {
        return Ok(Reply::Show(format!(
            "{} isn't installed with {}, so there's nothing to remove.",
            choice.title,
            manager.name()
        )));
    }
    if !ask(&format!(
        "Uninstall {} with {}?",
        choice.title,
        manager.name()
    )) {
        return Ok(Reply::Show("Okay, nothing was removed.".to_string()));
    }
    run_installer(&choice, Some(&manager), system, &args)?;
    Ok(Reply::Done(format!("Uninstalled {}.", choice.title)))
}

fn installer_name(system: &dyn System) -> &'static str {
    if system.os() == Os::Windows {
        "winget"
    } else {
        "Homebrew"
    }
}

fn update_all(system: &dyn System, ask: Ask) -> Result<Reply, ActionError> {
    let manager = package::manager(system)?;
    let question = format!(
        "Update everything installed with {} to its newest version? It can take a while.",
        manager.name()
    );
    if !ask(&question) {
        return Ok(Reply::Show("Okay, nothing was updated.".to_string()));
    }
    let steps: &[&[&str]] = match manager {
        // First fetch the list of newest versions, then upgrade to them.
        Manager::Homebrew(_) => &[&["update"], &["upgrade"]],
        Manager::Winget(_) => &[&["upgrade", "--all", "--source", "winget"]],
    };
    for step in steps {
        let worked = system.run(manager.program(), step).map_err(|error| {
            ActionError::Explained(format!("I couldn't start {}: {error}", manager.name()))
        })?;
        if !worked {
            return Err(ActionError::Explained(format!(
                "{} stopped with a problem. Its messages above say why.",
                manager.name()
            )));
        }
    }
    Ok(Reply::Done("Everything is up to date.".to_string()))
}

fn show_tools(os: Os) -> Reply {
    let mut lines = vec!["Tools Noob knows by name:".to_string()];
    for group in Group::ALL {
        lines.push(String::new());
        lines.push(group.title().to_string());
        for tool in TOOLS.iter().filter(|tool| tool.group == group) {
            let missing = match Package::for_tool(tool, os) {
                Ok(_) => "",
                Err(_) => "  (not on this computer)",
            };
            lines.push(format!(
                "  {:<14} {}: {}{missing}",
                tool.main_name(),
                tool.title,
                tool.about
            ));
        }
    }
    lines.push(String::new());
    lines.push("To install one, type: install git".to_string());
    lines.push("Looking for something else? Try: search tools video".to_string());
    Reply::Show(lines.join("\n"))
}

fn show_installed(system: &dyn System) -> Result<Reply, ActionError> {
    let manager = package::manager(system)?;
    let quiet = |args: &[&str]| {
        system
            .ask(manager.program(), args)
            .map(|answer| answer.text)
            .map_err(|error| {
                ActionError::Explained(format!("I couldn't ask {}: {error}", manager.name()))
            })
    };
    let text = match manager {
        Manager::Homebrew(_) => {
            let programs = package::brew_names(&quiet(&["list", "--formula", "-1"])?);
            let apps = package::brew_names(&quiet(&["list", "--cask", "-1"])?);
            format!(
                "Installed with Homebrew:\n  Terminal programs ({}): {}\n  Apps ({}): {}",
                programs.len(),
                or_none(&programs),
                apps.len(),
                or_none(&apps)
            )
        }
        Manager::Winget(_) => {
            let list = quiet(&["list", "--source", "winget", "--disable-interactivity"])?;
            format!("Installed with winget:\n{}", plain_text(&list).trim_end())
        }
    };
    Ok(Reply::Show(text))
}

fn or_none(names: &[String]) -> String {
    if names.is_empty() {
        "none yet".to_string()
    } else {
        names.join(", ")
    }
}

/// Output from another program can contain control characters that would mess up
/// the screen. Keep only plain text and line breaks.
fn plain_text(text: &str) -> String {
    text.lines()
        .map(|line| line.rsplit('\r').next().unwrap_or(line))
        .map(|line| line.chars().filter(|c| !c.is_control()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_installed(name: &str, system: &dyn System) -> Result<Reply, ActionError> {
    let tool = catalog::find(name);
    if tool.is_none() {
        check_name(name)?;
    }
    let title = tool.map_or(name, |tool| tool.title);
    let programs: Vec<&str> = tool.map_or_else(|| vec![name], |tool| tool.programs.to_vec());
    let manager = package::manager(system).ok();
    let package = match tool {
        Some(tool) => Package::for_tool(tool, system.os()).ok(),
        None => manager
            .as_ref()
            .and_then(|manager| package::exact_package(name, manager, system)),
    };
    let by_manager = match (&package, &manager) {
        (Some(package), Some(manager)) => {
            installed_with(package, manager, system).then(|| manager.name())
        }
        _ => None,
    };
    let found = programs
        .iter()
        .find_map(|program| find_program(program, system))
        .or_else(|| mac_app_path(tool, system));
    let answer = match (by_manager, found) {
        (Some(manager), Some(path)) => format!(
            "Yes, {title} is installed (with {manager}), at {}.",
            path.display()
        ),
        (Some(manager), None) => format!("Yes, {title} is installed (with {manager})."),
        (None, Some(path)) => format!("Yes, {title} is on your computer, at {}.", path.display()),
        (None, None) => format!(
            "No, {title} isn't installed. To install it, type: install {}",
            tool.map_or(name, |tool| tool.main_name())
        ),
    };
    Ok(Reply::Show(answer))
}

fn search_tools(text: &str, system: &dyn System) -> Result<Reply, ActionError> {
    if !package::valid_search(text) {
        return Err(ActionError::Explained(format!(
            "I can't search for \"{text}\". Try a simple word, like: search tools video"
        )));
    }
    let wanted = text.trim().to_lowercase();
    let curated: Vec<String> = TOOLS
        .iter()
        .filter(|tool| {
            tool.names.iter().any(|name| name.contains(&wanted))
                || tool.title.to_lowercase().contains(&wanted)
                || tool.about.to_lowercase().contains(&wanted)
        })
        .map(|tool| format!("{} ({})", tool.main_name(), tool.title))
        .collect();
    let mut lines = Vec::new();
    if !curated.is_empty() {
        lines.push(format!("From Noob's own list:\n{}", listed(&curated)));
    }
    // Noob's own list still helps when Homebrew or winget isn't there.
    match package::manager(system) {
        Ok(manager) => {
            let found = package::search(text.trim(), &manager, system)?;
            if !found.is_empty() {
                lines.push(format!("From {}:\n{}", manager.name(), listed(&found)));
            }
        }
        Err(error) if !curated.is_empty() => {
            lines.push(format!("(I only looked in Noob's own list. {error})"))
        }
        Err(error) => return Err(error),
    }
    if lines.is_empty() {
        return Ok(Reply::Show(format!(
            "Nothing matches \"{text}\". Check the spelling, or type: show tools"
        )));
    }
    lines.push("To install one, type install and its name.".to_string());
    Ok(Reply::Show(lines.join("\n")))
}

/// The real command for a tool command, for this kind of computer.
pub fn real_command(command: &ToolCommand, os: Os) -> Option<String> {
    let windows = os == Os::Windows;
    let package = |name: &str| match catalog::find(name) {
        Some(tool) => Package::for_tool(tool, os).ok(),
        None if !package::valid_name(name) => None,
        None if windows => Some(Package::Winget {
            id: name.to_string(),
            extra: Vec::new(),
        }),
        None => Some(Package::Formula(name.to_string())),
    };
    match command {
        ToolCommand::Install(name) => package(name).map(|package| package.real_install()),
        ToolCommand::Uninstall(name) => package(name).and_then(|package| package.real_uninstall()),
        ToolCommand::UpdateAll => Some(
            if windows {
                "winget upgrade --all"
            } else {
                "brew update && brew upgrade"
            }
            .to_string(),
        ),
        ToolCommand::ShowInstalled => {
            Some(if windows { "winget list" } else { "brew list" }.to_string())
        }
        ToolCommand::IsInstalled(name) => is_installed_command(name, os),
        ToolCommand::ShowTools => None,
        ToolCommand::SearchTools(text) => package::valid_search(text)
            .then(|| format!("{} search \"{}\"", installer(windows), text.trim())),
    }
}

fn installer(windows: bool) -> &'static str {
    if windows { "winget" } else { "brew" }
}

/// Programs are checked with `which` (or `Get-Command`); apps, which aren't programs
/// you type, are checked with the installer itself.
fn is_installed_command(name: &str, os: Os) -> Option<String> {
    let windows = os == Os::Windows;
    let program = match catalog::find(name) {
        Some(tool) => tool.programs.first().copied(),
        None => package::valid_name(name).then_some(name),
    };
    if let Some(program) = program {
        return Some(if windows {
            format!("Get-Command {program}")
        } else {
            format!("which {program}")
        });
    }
    let package = Package::for_tool(catalog::find(name)?, os).ok()?;
    let args = package.check_args()?;
    Some(format!("{} {}", installer(windows), args.join(" ")))
}
