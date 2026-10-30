//! Noob CLI — a beginner-friendly command line.
//!
//! Two ways to use it:
//!   noob make folder photos     run one phrase and finish
//!   noob                        open the noob> prompt and type many phrases

// The engine lives in the `noob_cli` library (src/lib.rs). These are just the
// terminal's own parts.
mod completer;
mod repl;
mod runner;

use std::env;
use std::process::ExitCode;

use runner::Outcome;

fn main() -> ExitCode {
    // The first argument is the program's own name, so skip it.
    // `args_os` + `to_string_lossy` copes with names that aren't valid text, instead of crashing.
    let words: Vec<String> = env::args_os()
        .skip(1)
        .map(|word| word.to_string_lossy().into_owned())
        .collect();

    if words.is_empty() {
        return repl::run();
    }

    let outcome = runner::run_words(&words);
    // When Noob's window opened this terminal for an installer, wait so the window
    // doesn't vanish before you've read what happened.
    if env::var_os(noob_cli::terminal::PAUSE_SETTING).is_some() {
        println!("\nPress Enter to close this window.");
        let mut enter = String::new();
        let _ = std::io::stdin().read_line(&mut enter); // any answer (or none) closes it
    }
    // A failure exit code lets scripts know something went wrong.
    match outcome {
        Outcome::Failed => ExitCode::FAILURE,
        Outcome::Done | Outcome::Quit => ExitCode::SUCCESS,
    }
}
