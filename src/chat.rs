//! The conversation shown in Noob's window: what you typed, Noob's answers, and any
//! "are you sure?" question waiting for a Yes or No button.
//!
//! There's no window code here at all, so it's tested like everything else.
//! `app/` only draws it.

use crate::actions;
use crate::command::{Command, ToolCommand};
use crate::complete::{self, Choice};
use crate::engine::{self, Line, Next, Outcome, Place};

pub const WELCOME: &str = "👋 Welcome to Noob!\n\
Type plain English below, like: make folder photos\n\
Press Tab to finish words, and the up arrow for earlier commands.\n\
Type help to see everything.";

/// The oldest messages are dropped after this many, so the window stays quick.
const MAX_ENTRIES: usize = 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Entry {
    /// Something you typed.
    You(String),
    /// Something Noob said.
    Noob(Line),
}

/// Opens a terminal window and runs Noob there. Installers can ask for your
/// password, and only a real terminal hands that safely to macOS or Windows.
/// (A "trait" lets the tests swap in a pretend version that opens nothing.)
pub trait Handoff {
    fn run_in_terminal(&self, words: &[String]) -> Result<(), String>;
}

/// A command for the window to run on a helper thread, so the window doesn't freeze
/// while it works (looking through a big folder can take a while).
pub struct Job(Command);

impl Job {
    /// Does the work. Safe to call on another thread; give the result to `Chat::finished`.
    pub fn run(self) -> Outcome {
        // Nothing the window runs asks in the middle of its work (installs are handed
        // to a terminal), so any such question would be answered "no".
        engine::finish(&self.0, &mut |_| false)
    }
}

struct Pending {
    question: String,
    command: Command,
}

pub struct Chat {
    entries: Vec<Entry>,
    pending: Option<Pending>,
    history: Vec<String>,
    /// Where the up/down arrows are in `history`, if they're being used.
    browsing: Option<usize>,
    quit: bool,
    /// A `Job` is running and its answer hasn't come back yet.
    busy: bool,
    handoff: Box<dyn Handoff>,
}

impl Chat {
    pub fn new(handoff: Box<dyn Handoff>) -> Chat {
        Chat {
            entries: vec![Entry::Noob(Line::Show(WELCOME.to_string()))],
            pending: None,
            history: Vec::new(),
            browsing: None,
            quit: false,
            busy: false,
            handoff,
        }
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The "are you sure?" question waiting for an answer, if there is one.
    pub fn question(&self) -> Option<&str> {
        self.pending
            .as_ref()
            .map(|pending| pending.question.as_str())
    }

    /// True once you've said bye.
    pub fn wants_to_quit(&self) -> bool {
        self.quit
    }

    /// The folder you're in, the way people say it ("~/photos").
    pub fn folder(&self) -> String {
        std::env::current_dir()
            .map(|folder| actions::short(&folder))
            .unwrap_or_default()
    }

    /// Just the folder's own name ("photos", or "~" for your home), for small spaces.
    pub fn folder_name(&self) -> String {
        let Ok(here) = std::env::current_dir() else {
            return String::new();
        };
        let short = actions::short(&here);
        if short == "~" {
            return short;
        }
        here.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or(short)
    }

    /// True while a `Job` is running. The window waits until it's done.
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    /// Takes a line you typed. If there's work to do, returns a `Job` for the window to
    /// run (then hand the result to `finished`).
    #[must_use = "the Job has to be run, or nothing happens"]
    pub fn send(&mut self, input: &str) -> Option<Job> {
        if input.trim().is_empty() || self.pending.is_some() || self.busy {
            return None;
        }
        self.add(Entry::You(input.trim().to_string()));
        // Lines starting with a space stay out of the history, like in the terminal.
        if !input.starts_with(' ') && self.history.last().map(String::as_str) != Some(input.trim())
        {
            self.history.push(input.trim().to_string());
        }
        self.browsing = None;
        match engine::understand(input, Place::Window) {
            Next::Finished(outcome) => {
                self.show(outcome);
                None
            }
            Next::Ask { question, command } => {
                self.pending = Some(Pending { question, command });
                None
            }
            Next::Run(command) => self.run(command),
        }
    }

    /// Answers the waiting "are you sure?" question. "Yes" may return a `Job` to run.
    #[must_use = "the Job has to be run, or nothing happens"]
    pub fn answer(&mut self, yes: bool) -> Option<Job> {
        let pending = self.pending.take()?;
        self.add(Entry::You(if yes { "Yes" } else { "No" }.to_string()));
        if yes {
            self.run(pending.command)
        } else {
            self.show(engine::declined());
            None
        }
    }

    /// The answer from a `Job` that was running.
    pub fn finished(&mut self, outcome: Outcome) {
        self.busy = false;
        self.show(outcome);
    }

    /// The command before this one in your history (the up arrow).
    pub fn older(&mut self) -> Option<String> {
        let at = match self.browsing {
            Some(0) => 0,
            Some(at) => at - 1,
            None => self.history.len().checked_sub(1)?,
        };
        self.browsing = Some(at);
        self.history.get(at).cloned()
    }

    /// The command after this one (the down arrow). Past the newest, it's an empty line.
    pub fn newer(&mut self) -> Option<String> {
        let at = self.browsing? + 1;
        if at >= self.history.len() {
            self.browsing = None;
            return Some(String::new());
        }
        self.browsing = Some(at);
        self.history.get(at).cloned()
    }

    /// Tab: choices for the end of what's typed, and where they go.
    pub fn complete(&self, typed: &str) -> (usize, Vec<Choice>) {
        complete::complete(typed)
    }

    fn run(&mut self, command: Command) -> Option<Job> {
        match command {
            Command::Clear => self.entries.clear(),
            Command::Tool(tool) if needs_terminal(&tool) => self.hand_off(&tool),
            command => {
                self.busy = true;
                return Some(Job(command));
            }
        }
        None
    }

    fn hand_off(&mut self, tool: &ToolCommand) {
        let words = terminal_words(tool);
        let typed = words.join(" ");
        match self.handoff.run_in_terminal(&words) {
            Ok(()) => {
                self.add(Entry::Noob(Line::Hint(format!(
                    "I opened a terminal window to run: noob {typed}\n\
                     Installers sometimes ask for your password there: that's macOS or \
                     Windows asking, not Noob. Come back here when it's finished."
                ))));
                if let Some(real) = Command::Tool(tool.clone()).real_command() {
                    self.add(Entry::Noob(Line::Real(real)));
                }
            }
            Err(problem) => self.add(Entry::Noob(Line::Problem(format!(
                "I couldn't open a terminal window ({problem}). Open one yourself and type: noob {typed}"
            )))),
        }
    }

    fn show(&mut self, outcome: Outcome) {
        for line in outcome.lines {
            self.add(Entry::Noob(line));
        }
        if outcome.quit {
            self.quit = true;
        }
    }

    fn add(&mut self, entry: Entry) {
        self.entries.push(entry);
        if self.entries.len() > MAX_ENTRIES {
            let extra = self.entries.len() - MAX_ENTRIES;
            self.entries.drain(..extra);
        }
    }
}

/// Installing, uninstalling and updating can ask for a password, so they happen in a
/// terminal window. Looking things up (`show tools`, `is git installed`) stays here.
fn needs_terminal(tool: &ToolCommand) -> bool {
    matches!(
        tool,
        ToolCommand::Install(_) | ToolCommand::Uninstall(_) | ToolCommand::UpdateAll
    )
}

/// The words to type after `noob` to run this in a terminal.
fn terminal_words(tool: &ToolCommand) -> Vec<String> {
    let words: Vec<&str> = match tool {
        ToolCommand::Install(name) => vec!["install", name],
        ToolCommand::Uninstall(name) => vec!["uninstall", name],
        ToolCommand::UpdateAll => vec!["update", "everything"],
        ToolCommand::ShowTools => vec!["show", "tools"],
        ToolCommand::ShowInstalled => vec!["show", "installed"],
        ToolCommand::IsInstalled(name) => vec!["is", name, "installed"],
        ToolCommand::SearchTools(text) => vec!["search", "tools", text],
    };
    words.into_iter().map(str::to_string).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// A pretend handoff that writes down what it would have run.
    #[derive(Clone, Default)]
    struct FakeHandoff {
        ran: Rc<RefCell<Vec<Vec<String>>>>,
        broken: bool,
    }

    impl Handoff for FakeHandoff {
        fn run_in_terminal(&self, words: &[String]) -> Result<(), String> {
            if self.broken {
                return Err("no terminal".to_string());
            }
            self.ran.borrow_mut().push(words.to_vec());
            Ok(())
        }
    }

    fn chat() -> (Chat, FakeHandoff) {
        let handoff = FakeHandoff::default();
        (Chat::new(Box::new(handoff.clone())), handoff)
    }

    /// Types a line and, like the window, runs any work it gives back.
    fn type_line(chat: &mut Chat, line: &str) {
        if let Some(job) = chat.send(line) {
            assert!(chat.is_busy());
            chat.finished(job.run());
        }
    }

    fn last(chat: &Chat) -> &Entry {
        chat.entries().last().expect("something was said")
    }

    #[test]
    fn starts_with_a_welcome() {
        let (chat, _) = chat();
        assert!(
            matches!(&chat.entries()[0], Entry::Noob(Line::Show(text)) if text.contains("Welcome"))
        );
    }

    #[test]
    fn shows_what_you_typed_then_the_answer_and_real_command() {
        let (mut chat, _) = chat();
        type_line(&mut chat, "where am i");
        let entries = chat.entries();
        assert_eq!(entries[1], Entry::You("where am i".to_string()));
        assert!(
            matches!(&entries[2], Entry::Noob(Line::Show(text)) if text.starts_with("You are in"))
        );
        assert_eq!(entries[3], Entry::Noob(Line::Real("pwd".to_string())));
    }

    #[test]
    fn typos_get_a_hint() {
        let (mut chat, _) = chat();
        type_line(&mut chat, "shwo files");
        assert!(
            matches!(last(&chat), Entry::Noob(Line::Hint(text)) if text.contains("show files"))
        );
    }

    #[test]
    fn risky_commands_wait_for_yes_or_no() {
        let (mut chat, _) = chat();
        type_line(&mut chat, "delete Cargo.toml");
        assert!(
            chat.question()
                .is_some_and(|question| question.contains("Cargo.toml"))
        );

        type_line(&mut chat, "show files"); // ignored until the question is answered
        assert!(chat.question().is_some());

        assert!(chat.answer(false).is_none());
        assert_eq!(chat.question(), None);
        assert!(matches!(last(&chat), Entry::Noob(Line::Show(text)) if text.starts_with("Okay")));
        assert!(
            std::path::Path::new("Cargo.toml").exists(),
            "no means nothing happened"
        );
    }

    #[test]
    fn saying_yes_runs_it() {
        let file = std::env::temp_dir().join(format!("noob-chat-yes-{}.txt", std::process::id()));
        std::fs::write(&file, "bye").unwrap();
        let (mut chat, _) = chat();
        type_line(&mut chat, &format!("delete \"{}\"", file.display()));
        let job = chat.answer(true).expect("yes gives work to do");
        chat.finished(job.run());
        // (Tests use a stand-in for the Trash, so this really is gone.)
        assert!(!file.exists());
        assert!(matches!(last(&chat), Entry::Noob(Line::Real(_))));
    }

    #[test]
    fn nothing_new_starts_while_busy() {
        let (mut chat, _) = chat();
        let job = chat.send("where am i").expect("work to do");
        assert!(chat.send("show files").is_none(), "ignored while busy");
        chat.finished(job.run());
        assert!(!chat.is_busy());
    }

    #[test]
    fn installs_open_a_terminal_window() {
        let (mut chat, handoff) = chat();
        type_line(&mut chat, "install git");
        assert_eq!(
            *handoff.ran.borrow(),
            vec![vec!["install".to_string(), "git".to_string()]]
        );
        assert!(chat.entries().iter().any(|entry| matches!(entry, Entry::Noob(Line::Hint(text)) if text.contains("noob install git"))));
    }

    #[test]
    fn a_missing_terminal_is_explained() {
        let handoff = FakeHandoff {
            broken: true,
            ..FakeHandoff::default()
        };
        let mut chat = Chat::new(Box::new(handoff));
        type_line(&mut chat, "update everything");
        assert!(
            matches!(last(&chat), Entry::Noob(Line::Problem(text)) if text.contains("noob update everything"))
        );
    }

    #[test]
    fn looking_things_up_stays_in_the_window() {
        let (mut chat, handoff) = chat();
        type_line(&mut chat, "show tools");
        assert!(handoff.ran.borrow().is_empty());
        assert!(
            matches!(&chat.entries()[2], Entry::Noob(Line::Show(text)) if text.contains("Tools Noob knows"))
        );
    }

    #[test]
    fn clear_empties_the_window_and_bye_closes_it() {
        let (mut chat, _) = chat();
        type_line(&mut chat, "clear");
        assert!(chat.entries().is_empty());
        type_line(&mut chat, "bye");
        assert!(chat.wants_to_quit());
    }

    #[test]
    fn arrows_walk_through_earlier_commands() {
        let (mut chat, _) = chat();
        type_line(&mut chat, "show files");
        type_line(&mut chat, "where am i");
        type_line(&mut chat, " secret line");
        assert_eq!(chat.older().as_deref(), Some("where am i"));
        assert_eq!(chat.older().as_deref(), Some("show files"));
        assert_eq!(
            chat.older().as_deref(),
            Some("show files"),
            "stops at the oldest"
        );
        assert_eq!(chat.newer().as_deref(), Some("where am i"));
        assert_eq!(
            chat.newer().as_deref(),
            Some(""),
            "past the newest is an empty line"
        );
    }

    #[test]
    fn every_tool_command_has_terminal_words_noob_understands() {
        let commands = [
            ToolCommand::Install("build tools".to_string()),
            ToolCommand::Uninstall("vlc".to_string()),
            ToolCommand::UpdateAll,
        ];
        for tool in commands {
            let words = terminal_words(&tool);
            let reparsed = engine::understand_words(&words);
            assert!(
                matches!(reparsed, Next::Run(Command::Tool(ref again)) if *again == tool),
                "{words:?}"
            );
        }
    }
}
