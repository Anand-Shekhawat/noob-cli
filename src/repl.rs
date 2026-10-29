//! Prompt mode: run `noob` on its own, then type one phrase after another.
//! ("REPL" means Read-Eval-Print Loop: read a line, run it, print, repeat.)
//!
//! The `rustyline` crate handles the keyboard: up-arrow brings back earlier
//! commands, and Tab asks `completer.rs` what could come next.

use std::io;
use std::path::Path;
use std::process::ExitCode;

use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::{CompletionType, Config, Editor};

use crate::actions;
use crate::completer::NoobHelper;
use crate::runner::{self, Outcome};
use crate::style;

const WELCOME: &str = "\
👋 Welcome to Noob CLI!
Type plain English, like: make folder photos
Press Tab to finish words, and the up arrow for earlier commands.
Type help to see everything, or bye to leave.";

/// Earlier commands are saved in this file in your home folder, like `~/.zsh_history`.
const HISTORY_FILE: &str = ".noob_history";
const HISTORY_SIZE: usize = 1000;

type NoobEditor = Editor<NoobHelper, FileHistory>;

pub fn run() -> ExitCode {
    println!("{WELCOME}");
    let mut editor = match new_editor() {
        Ok(editor) => editor,
        Err(error) => {
            eprintln!("✗ I couldn't start the noob prompt: {error}");
            return ExitCode::FAILURE;
        }
    };

    let history = std::env::home_dir().map(|home| home.join(HISTORY_FILE));
    if let Some(path) = &history {
        load_history(&mut editor, path);
    }
    let code = prompt_loop(&mut editor);
    if let Some(path) = &history
        && let Err(error) = editor.save_history(path)
    {
        eprintln!("⚠ I couldn't save your command history: {error}");
    }
    code
}

fn new_editor() -> rustyline::Result<NoobEditor> {
    let config = Config::builder()
        .max_history_size(HISTORY_SIZE)?
        .history_ignore_dups(true)?
        .completion_type(CompletionType::List)
        .build();
    let mut editor = Editor::with_config(config)?;
    editor.set_helper(Some(NoobHelper));
    Ok(editor)
}

fn load_history(editor: &mut NoobEditor, path: &Path) {
    match editor.load_history(path) {
        Ok(()) => {}
        // No history file yet just means this is your first time.
        Err(ReadlineError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => eprintln!("⚠ I couldn't load your command history: {error}"),
    }
}

fn prompt_loop(editor: &mut NoobEditor) -> ExitCode {
    loop {
        let plain = format!("noob {}> ", here_label());
        let coloured = style::prompt(&plain);
        // rustyline takes the plain prompt (to measure it) and the coloured one (to show it).
        match editor.readline(&(plain.as_str(), coloured.as_str())) {
            Ok(line) => {
                if !line.trim().is_empty()
                    && let Err(error) = editor.add_history_entry(line.as_str())
                {
                    eprintln!("⚠ I couldn't remember that command: {error}");
                }
                if let Outcome::Quit = runner::run_line(&line) {
                    return ExitCode::SUCCESS;
                }
            }
            // Ctrl+C clears the line, like in a normal terminal.
            Err(ReadlineError::Interrupted) => println!("(To leave Noob, type bye.)"),
            // Ctrl+D means "I'm done".
            Err(ReadlineError::Eof) => {
                println!();
                return ExitCode::SUCCESS;
            }
            Err(error) => {
                eprintln!("✗ I couldn't read what you typed: {error}");
                return ExitCode::FAILURE;
            }
        }
    }
}

fn here_label() -> String {
    std::env::current_dir()
        .map(|folder| prompt_label(&folder))
        .unwrap_or_default()
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
