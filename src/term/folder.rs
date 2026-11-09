//! Which folder your shell is in right now.
//!
//! After you type `cd photos` in the terminal, the shell is in `photos`, but
//! Noob (a different program) isn't. The Noob bar needs the shell's folder, so
//! `delete notes.txt` means the `notes.txt` you can see. The operating system
//! knows every program's folder; we ask it:
//!   - on a Mac, with the built-in `lsof` tool ("list open files");
//!   - on Linux, by reading `/proc/<number>/cwd`;
//!   - on Windows there's no simple way, so we don't know (`None`).

use std::path::PathBuf;

/// The folder the program with this process number is in, if we can find out.
pub fn of_process(pid: u32) -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        let output = std::process::Command::new("lsof")
            .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
            .output()
            .ok()?;
        parse_lsof(&String::from_utf8_lossy(&output.stdout))
    } else if cfg!(target_os = "linux") {
        std::fs::read_link(format!("/proc/{pid}/cwd")).ok()
    } else {
        None
    }
}

/// `lsof -Fn` prints one field per line; the folder's line starts with `n`.
fn parse_lsof(output: &str) -> Option<PathBuf> {
    output
        .lines()
        .find_map(|line| line.strip_prefix('n'))
        .filter(|path| path.starts_with('/'))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_folder_out_of_lsofs_answer() {
        let output = "p123\nfcwd\nn/Users/anand/photos\n";
        assert_eq!(
            parse_lsof(output),
            Some(PathBuf::from("/Users/anand/photos"))
        );
    }

    #[test]
    fn an_empty_or_odd_answer_means_we_do_not_know() {
        assert_eq!(parse_lsof(""), None);
        assert_eq!(parse_lsof("p123\nfcwd\n"), None);
        assert_eq!(parse_lsof("nnot-a-path\n"), None);
    }

    #[cfg(unix)]
    #[test]
    fn finds_the_folder_a_running_program_is_in() {
        let folder = std::env::temp_dir().join(format!("noob-folder-{}", std::process::id()));
        std::fs::create_dir_all(&folder).expect("make test folder");
        let mut sleeper = std::process::Command::new("sleep")
            .arg("10")
            .current_dir(&folder)
            .spawn()
            .expect("start sleep");
        // A Mac's temp folder is reached through a shortcut (/var → /private/var),
        // so both sides are turned into their real, full paths before comparing.
        let found = of_process(sleeper.id()).and_then(|path| path.canonicalize().ok());
        let expected = folder.canonicalize().ok();
        let _ = sleeper.kill();
        let _ = sleeper.wait();
        let _ = std::fs::remove_dir_all(&folder);
        assert!(found.is_some(), "the folder is found");
        assert_eq!(found, expected);
    }
}
