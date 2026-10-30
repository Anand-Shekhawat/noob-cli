//! Opening a terminal window that runs Noob there. Noob's window uses this for
//! installers, which may ask for a password that only a real terminal can pass
//! safely to macOS or Windows.

use std::path::{Path, PathBuf};

use crate::chat::Handoff;

/// When this setting is present, the terminal `noob` waits for Enter before its window
/// closes, so you can read what happened.
pub const PAUSE_SETTING: &str = "NOOB_PAUSE_AT_END";

pub struct TerminalWindow;

impl Handoff for TerminalWindow {
    fn run_in_terminal(&self, words: &[String]) -> Result<(), String> {
        open(&noob_program(), words)
    }
}

/// The terminal `noob` that comes with the app (it sits right next to it), or
/// failing that, the one the terminal would find.
fn noob_program() -> PathBuf {
    let name = if cfg!(windows) { "noob.exe" } else { "noob" };
    std::env::current_exe()
        .ok()
        .map(|app| app.with_file_name(name))
        .filter(|program| program.exists())
        .unwrap_or_else(|| PathBuf::from(name))
}

/// On a Mac: a ".command" file is a little script the Terminal app opens and runs.
/// (Asking Terminal directly would need an extra permission from macOS.)
#[cfg(target_os = "macos")]
fn open(noob: &Path, words: &[String]) -> Result<(), String> {
    let script =
        command_file(noob, words).map_err(|error| format!("couldn't write the script: {error}"))?;
    let status = std::process::Command::new("open")
        .arg(&script)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("the Terminal app didn't start".to_string())
    }
}

#[cfg(target_os = "macos")]
fn command_file(noob: &Path, words: &[String]) -> std::io::Result<PathBuf> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static MADE: AtomicUsize = AtomicUsize::new(0);
    let made = MADE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("noob-{}-{made}.command", std::process::id()));
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true) // never reuse (or follow) a file that's already there
        .mode(0o700) // only you can read or run it
        .open(&path)?
        .write_all(script(noob, words).as_bytes())?;
    Ok(path)
}

/// The script's text. Every word is quoted, so nothing in a name can become a command.
#[cfg(any(test, target_os = "macos"))]
fn script(noob: &Path, words: &[String]) -> String {
    let typed: Vec<String> = std::iter::once(noob.display().to_string())
        .chain(words.iter().cloned())
        .map(|word| sh_quote(&word))
        .collect();
    format!(
        "#!/bin/bash\n# Made by Noob. It deletes itself after starting.\nrm -f \"$0\"\ncd ~ || exit\n{PAUSE_SETTING}=1 {}\n",
        typed.join(" ")
    )
}

/// Inside 'single quotes' the Mac terminal takes everything literally, except a '
/// itself, which is written as '\''.
#[cfg(any(test, target_os = "macos"))]
fn sh_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// On Windows: start `noob.exe` in a brand new console window.
#[cfg(windows)]
fn open(noob: &Path, words: &[String]) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
    std::process::Command::new(noob)
        .args(words)
        .env(PAUSE_SETTING, "1")
        .creation_flags(CREATE_NEW_CONSOLE)
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// On Linux: most desktops provide `x-terminal-emulator`, which opens the usual terminal.
#[cfg(all(unix, not(target_os = "macos")))]
fn open(noob: &Path, words: &[String]) -> Result<(), String> {
    std::process::Command::new("x-terminal-emulator")
        .arg("-e")
        .arg(noob)
        .args(words)
        .env(PAUSE_SETTING, "1")
        .spawn()
        .map(|_| ())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mac_script_quotes_every_word() {
        // A name full of shell tricks must stay one harmless, quoted word.
        let words = vec!["install".to_string(), "it's; echo oops".to_string()];
        let text = script(
            Path::new("/Applications/Noob.app/Contents/MacOS/noob"),
            &words,
        );
        assert!(
            text.contains(
                r"'/Applications/Noob.app/Contents/MacOS/noob' 'install' 'it'\''s; echo oops'"
            ),
            "{text}"
        );
        assert!(text.contains("NOOB_PAUSE_AT_END=1"));
        assert!(
            text.contains("rm -f \"$0\""),
            "the script cleans up after itself"
        );
    }
}
