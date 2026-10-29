//! Prompt mode: run `noob` on its own, then type one phrase after another.
//! ("REPL" means Read-Eval-Print Loop: read a line, run it, print, repeat.)

use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;

use crate::actions;
use crate::runner::{self, Outcome};

const WELCOME: &str = "\
👋 Welcome to Noob CLI!
Type plain English, like: make folder photos
Type help to see everything, or bye to leave.";

pub fn run() -> ExitCode {
    println!("{WELCOME}");
    loop {
        let here = std::env::current_dir()
            .map(|folder| prompt_label(&folder))
            .unwrap_or_default();
        print!("noob {here}> ");
        // `print!` waits for a newline before showing text; `flush` shows it right now.
        if let Err(error) = io::stdout().flush() {
            eprintln!("✗ I couldn't write to the screen: {error}");
            return ExitCode::FAILURE;
        }

        let mut line = String::new();
        match io::stdin().read_line(&mut line) {
            Ok(0) => {
                // 0 bytes means input ended (you pressed Ctrl+D).
                println!();
                return ExitCode::SUCCESS;
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("✗ I couldn't read what you typed: {error}");
                return ExitCode::FAILURE;
            }
        }

        if let Outcome::Quit = runner::run_line(&line) {
            return ExitCode::SUCCESS;
        }
    }
}

/// The prompt shows just the current folder's name, like the Mac terminal does.
/// (`where am i` shows the full path.)
fn prompt_label(folder: &Path) -> String {
    let short = actions::short(folder);
    if short == "~" {
        return short;
    }
    folder
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or(short)
}
