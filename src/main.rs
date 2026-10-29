//! Noob CLI — a beginner-friendly command line.
//!
//! Two ways to use it:
//!   noob make folder photos     run one phrase and finish
//!   noob                        open the noob> prompt and type many phrases

mod actions;
mod command;
mod help;
mod parser;
mod repl;
mod runner;
mod suggest;
mod table;

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

    // A failure exit code lets scripts know something went wrong.
    match runner::run_words(&words) {
        Outcome::Failed => ExitCode::FAILURE,
        Outcome::Done | Outcome::Quit => ExitCode::SUCCESS,
    }
}
