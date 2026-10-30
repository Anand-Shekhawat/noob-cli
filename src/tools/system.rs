//! How Noob talks to other programs, like Homebrew and winget.
//!
//! Everything goes through the `System` trait. A "trait" is a promise: "anything that
//! is a System can do these things". The real program uses `RealSystem`, which really
//! runs them. The tests use a pretend system that only writes down what would have run,
//! so testing never installs anything.

use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The kind of computer Noob is running on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Os {
    Mac,
    Windows,
    Linux,
}

impl Os {
    pub fn current() -> Os {
        if cfg!(target_os = "macos") {
            Os::Mac
        } else if cfg!(windows) {
            Os::Windows
        } else {
            Os::Linux
        }
    }
}

/// What a program printed when it was run quietly.
pub struct Answer {
    pub worked: bool,
    pub text: String,
}

pub trait System {
    fn os(&self) -> Os;
    /// Where a program is, if the terminal can find it (it's in PATH).
    fn find(&self, program: &str) -> Option<PathBuf>;
    fn exists(&self, path: &Path) -> bool;
    /// Your home folder.
    fn home(&self) -> Option<PathBuf>;
    /// Runs a program on screen: you see everything it prints, and it can ask you
    /// questions (like your password) itself. Noob never sees what you type to it.
    /// Returns `true` if it worked.
    fn run(&self, program: &Path, args: &[&str]) -> io::Result<bool>;
    /// Runs a program quietly and returns what it printed.
    fn ask(&self, program: &Path, args: &[&str]) -> io::Result<Answer>;
}

pub struct RealSystem;

impl System for RealSystem {
    fn os(&self) -> Os {
        Os::current()
    }

    fn find(&self, program: &str) -> Option<PathBuf> {
        find_in(program, env::var_os("PATH"))
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn home(&self) -> Option<PathBuf> {
        env::home_dir()
    }

    fn run(&self, program: &Path, args: &[&str]) -> io::Result<bool> {
        Ok(Command::new(program).args(args).status()?.success())
    }

    fn ask(&self, program: &Path, args: &[&str]) -> io::Result<Answer> {
        // No keyboard: a quiet question must never sit waiting for an answer.
        let output = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .output()?;
        Ok(Answer {
            worked: output.status.success(),
            text: String::from_utf8_lossy(&output.stdout).into_owned(),
        })
    }
}

/// Looks in every folder listed in PATH, the way the terminal does.
fn find_in(program: &str, path: Option<OsString>) -> Option<PathBuf> {
    let folders = path?;
    env::split_paths(&folders)
        .flat_map(|folder| possible_files(&folder, program))
        .find(|file| file.is_file())
}

/// On Windows, programs end in ".exe" (or ".cmd"), but you type them without it.
fn possible_files(folder: &Path, program: &str) -> Vec<PathBuf> {
    if cfg!(windows) {
        [".exe", ".cmd", ".bat"]
            .iter()
            .map(|ending| folder.join(format!("{program}{ending}")))
            .collect()
    } else {
        vec![folder.join(program)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_programs_in_the_folders_listed_in_path() {
        let folder = env::temp_dir().join(format!("noob-path-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let name = if cfg!(windows) { "tool.exe" } else { "tool" };
        std::fs::write(folder.join(name), "").unwrap();

        let path = env::join_paths([folder.clone()]).unwrap();
        assert_eq!(find_in("tool", Some(path.clone())), Some(folder.join(name)));
        assert_eq!(find_in("missing", Some(path)), None);
        assert_eq!(find_in("tool", None), None);
        let _ = std::fs::remove_dir_all(&folder);
    }
}
