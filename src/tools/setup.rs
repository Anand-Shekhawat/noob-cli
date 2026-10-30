//! The step people always miss after installing Homebrew: telling the terminal where it is.
//!
//! Homebrew lives in a folder the terminal doesn't look in yet. One line in a settings
//! file (`~/.zprofile` on a Mac) fixes that for every new terminal window.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::Ask;
use super::package::{homebrew_location, is_standard_homebrew};
use super::system::{Os, System};

/// Says what's left to do, after asking before changing your settings file.
/// It never fails: the install itself already worked, so problems here become advice.
pub fn after_homebrew(system: &dyn System, ask: Ask) -> String {
    let Some(brew) = homebrew_location(system) else {
        return "But I can't find Homebrew now, so it may not have finished. Its messages above say why.".to_string();
    };
    if system.find("brew").is_some() {
        return String::new(); // the terminal already knows where it is
    }
    // Only Homebrew's own places go into a settings file the terminal runs every time.
    if !is_standard_homebrew(&brew) {
        return format!(
            "Homebrew is at {}, which isn't where it usually goes, so I left your settings alone.",
            brew.display()
        );
    }
    let Some(settings) = settings_file(system) else {
        return String::new();
    };
    let line = shellenv_line(&brew);
    match fs::read_to_string(&settings) {
        Ok(text) if text.contains(&line) => return new_window(),
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        // If it can't be read, adding to it blindly could add the line twice.
        Err(error) => return manual(&settings, &line, &format!("I couldn't read it ({error}).")),
    }
    let question = format!(
        "Your terminal doesn't know where Homebrew is yet. Add one line to {} so it does?",
        settings.display()
    );
    if !ask(&question) {
        return manual(&settings, &line, "");
    }
    match add_line(&settings, &line) {
        Ok(()) => new_window(),
        Err(error) => manual(
            &settings,
            &line,
            &format!("I couldn't change it ({error})."),
        ),
    }
}

fn new_window() -> String {
    "Open a new terminal window to start using brew.".to_string()
}

fn manual(settings: &Path, line: &str, problem: &str) -> String {
    let problem = if problem.is_empty() {
        String::new()
    } else {
        format!("{problem} ")
    };
    format!(
        "{problem}To set it up yourself, add this line to {}: {line}",
        settings.display()
    )
}

/// The settings file the terminal reads when a new window opens.
fn settings_file(system: &dyn System) -> Option<PathBuf> {
    let home = system.home()?;
    Some(match system.os() {
        Os::Linux => home.join(".bashrc"),
        Os::Mac | Os::Windows => home.join(".zprofile"),
    })
}

/// The line Homebrew itself recommends adding.
fn shellenv_line(brew: &Path) -> String {
    format!(r#"eval "$({} shellenv)""#, brew.display())
}

fn add_line(settings: &Path, line: &str) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(settings)?;
    writeln!(
        file,
        "\n# Added by Noob, so the terminal can find Homebrew\n{line}"
    )
}
