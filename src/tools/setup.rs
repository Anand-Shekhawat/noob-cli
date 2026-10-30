//! The step people always miss after installing Homebrew: telling the terminal where it is.
//!
//! Homebrew lives in a folder the terminal doesn't look in yet. One line in a settings
//! file (`~/.zprofile` on a Mac) fixes that for every new terminal window.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::package::homebrew_location;
use super::system::{Os, System};
use crate::actions::ActionError;

/// Says what's left to do, after asking before changing your settings file.
pub fn after_homebrew(
    system: &dyn System,
    ask: &mut dyn FnMut(&str) -> bool,
) -> Result<String, ActionError> {
    let Some(brew) = homebrew_location(system) else {
        return Ok("But I can't find Homebrew now, so it may not have finished. Its messages above say why.".to_string());
    };
    if system.find("brew").is_some() {
        return Ok(String::new()); // the terminal already knows where it is
    }
    let Some(settings) = settings_file(system) else {
        return Ok(String::new());
    };
    let line = shellenv_line(&brew);
    let already = fs::read_to_string(&settings).is_ok_and(|text| text.contains(&line));
    if !already {
        let question = format!(
            "Your terminal doesn't know where Homebrew is yet. Add one line to {} so it does?",
            settings.display()
        );
        if !ask(&question) {
            return Ok(format!(
                "To set it up yourself later, add this line to {}: {line}",
                settings.display()
            ));
        }
        add_line(&settings, &line)?;
    }
    Ok("Open a new terminal window to start using brew.".to_string())
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

fn add_line(settings: &Path, line: &str) -> Result<(), ActionError> {
    let label = settings.display().to_string();
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(settings)
        .and_then(|mut file| {
            writeln!(
                file,
                "\n# Added by Noob, so the terminal can find Homebrew\n{line}"
            )
        })
        .map_err(|error| ActionError::Explained(format!("I couldn't change {label}: {error}")))
}
