//! The Noob bar along the bottom of the window.
//!
//! You type plain English; Noob writes the real command into the terminal's
//! prompt and you press Enter there to run it (see `noob_cli::bar` for how it
//! decides). Risky things ask first, with Yes and No. Phrases with no real
//! command, like `help`, are answered right here in the bar.
//!
//! Cmd+K (Ctrl+K on Windows, Ctrl+Shift+K on Linux) jumps between the bar and
//! the terminal; Esc in the bar goes back to the terminal.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use eframe::egui::{self, Event, Key, Modifiers, RichText};
use noob_cli::bar::{self, Setup, Then};
use noob_cli::complete::{self, Choice};
use noob_cli::engine::{self, Line, Outcome};

use crate::terminal::{TerminalView, Theme};

/// Long answers (like `help`) scroll inside the bar instead of pushing the
/// terminal off the screen.
const MAX_ANSWER_HEIGHT: f32 = 220.0;
const INPUT_TEXT_SIZE: f32 = 15.0;

pub struct NoobBar {
    setup: Setup,
    typed: String,
    history: History,
    /// Tab choices waiting to be picked, and where in `typed` they go.
    choices: Vec<Choice>,
    choices_from: usize,
    /// A risky plan waiting for Yes or No.
    waiting: Option<(String, Then)>,
    /// The last line sent, put back in the box if the terminal was busy.
    last_line: String,
    /// What Noob said last.
    answer: Vec<Line>,
    /// What the command just typed into the terminal does, in plain words.
    about: Option<String>,
    /// Where the answer to something Noob is doing itself will arrive.
    working: Option<Receiver<Outcome>>,
    wants_focus: bool,
    /// Noob's starting folder, used when the shell's folder can't be found out.
    start_folder: Option<std::path::PathBuf>,
    /// The Enter that sent a line to the bar is still held down: its repeats
    /// must not reach the terminal and run the command.
    swallow_enter: bool,
}

fn input_id() -> egui::Id {
    egui::Id::unique("noob-bar-input")
}

impl NoobBar {
    pub fn new() -> Self {
        NoobBar {
            setup: Setup::detect(),
            typed: String::new(),
            history: History::default(),
            choices: Vec::new(),
            choices_from: 0,
            waiting: None,
            last_line: String::new(),
            answer: Vec::new(),
            about: None,
            working: None,
            wants_focus: false,
            start_folder: std::env::current_dir().ok(),
            swallow_enter: false,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, terminal: &mut TerminalView, theme: &Theme) {
        self.collect();
        if self.swallow_enter {
            self.swallow_held_enter(ui);
        }
        if shortcut_pressed(ui) {
            if ui.memory(|memory| memory.has_focus(input_id())) {
                terminal.focus();
                ui.memory_mut(|memory| memory.surrender_focus(input_id()));
            } else {
                self.wants_focus = true;
            }
        }
        if !self.answer.is_empty() {
            self.answer_area(ui, theme);
        }
        if self.waiting.is_some() {
            self.question_row(ui, terminal, theme);
        } else if self.working.is_some() {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Working…").color(theme.fg));
            });
        } else {
            if !self.choices.is_empty() {
                self.choice_row(ui, theme);
            }
            self.input_row(ui, terminal, theme);
        }
    }

    /// Drops Enter presses until the Enter key is let go.
    fn swallow_held_enter(&mut self, ui: &egui::Ui) {
        ui.input_mut(|input| {
            let released = input.events.iter().any(|event| {
                matches!(
                    event,
                    Event::Key {
                        key: Key::Enter,
                        pressed: false,
                        ..
                    }
                )
            });
            input.events.retain(|event| {
                !matches!(
                    event,
                    Event::Key {
                        key: Key::Enter,
                        ..
                    }
                )
            });
            if released || !input.key_down(Key::Enter) {
                self.swallow_enter = false;
            }
        });
    }

    /// Moves Noob to the shell's folder, so names (and Tab) mean the files you
    /// see there. If that can't be found out, Noob's starting folder is used.
    fn follow_shell_folder(&self, terminal: &TerminalView) {
        if let Some(folder) = terminal
            .shell_folder()
            .or_else(|| self.start_folder.clone())
        {
            let _ = std::env::set_current_dir(folder);
        }
    }

    // ── Typing ──────────────────────────────────────────────────────────────

    fn input_row(&mut self, ui: &mut egui::Ui, terminal: &mut TerminalView, theme: &Theme) {
        if ui.memory(|memory| memory.has_focus(input_id())) {
            self.special_keys(ui, terminal);
        }
        let mut submitted = None;
        ui.horizontal(|ui| {
            // Same size as the text, so the two sit on one line.
            ui.label(
                RichText::new("›")
                    .size(INPUT_TEXT_SIZE)
                    .strong()
                    .color(theme.cursor),
            );
            let hint_width = 90.0;
            let edit = egui::TextEdit::singleline(&mut self.typed)
                .id(input_id())
                .frame(egui::Frame::NONE)
                .hint_text("Tell Noob what to do, like: make folder photos")
                .font(egui::FontId::proportional(INPUT_TEXT_SIZE))
                .text_color(theme.fg)
                .desired_width(ui.available_width() - hint_width)
                .lock_focus(true); // Tab finishes words instead of leaving the box
            let response = ui.add(edit);
            if response.changed() {
                self.choices.clear();
            }
            if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
                submitted = Some(std::mem::take(&mut self.typed));
                self.swallow_enter = true;
                // This Enter belongs to the bar. The terminal takes the keyboard
                // straight away, and must not see it too, or it would run the
                // command before you had a chance to read it.
                ui.input_mut(|input| {
                    input.events.retain(|event| {
                        !matches!(
                            event,
                            Event::Key {
                                key: Key::Enter,
                                ..
                            }
                        )
                    });
                });
            }
            if std::mem::take(&mut self.wants_focus) {
                response.request_focus();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(shortcut_name())
                        .size(12.0)
                        .color(theme.ansi[8]),
                )
                .on_hover_text("Jump between the terminal and the Noob bar");
            });
        });
        if let Some(line) = submitted {
            self.submit(ui.ctx(), &line, terminal);
        }
    }

    /// Up/down for earlier lines, Tab to finish words. Taken before the text
    /// box sees them, so it doesn't move its cursor instead.
    fn special_keys(&mut self, ui: &egui::Ui, terminal: &TerminalView) {
        let (up, down, tab) = ui.input_mut(|input| {
            (
                input.consume_key(Modifiers::NONE, Key::ArrowUp),
                input.consume_key(Modifiers::NONE, Key::ArrowDown),
                input.consume_key(Modifiers::NONE, Key::Tab),
            )
        });
        if up && let Some(line) = self.history.older() {
            self.set_typed(ui, line);
        }
        if down && let Some(line) = self.history.newer() {
            self.set_typed(ui, line);
        }
        if tab {
            self.follow_shell_folder(terminal);
            self.finish_word(ui);
        }
    }

    /// Tab: finish the word at the end of what's typed.
    fn finish_word(&mut self, ui: &egui::Ui) {
        let (from, choices) = complete::complete(&self.typed);
        match choices.as_slice() {
            [] => {}
            [only] => {
                let finished = format!("{}{}", &self.typed[..from], only.replacement);
                self.set_typed(ui, finished);
            }
            _ => {
                self.choices = choices;
                self.choices_from = from;
            }
        }
    }

    fn choice_row(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Pick one:").color(theme.ansi[8]));
            for choice in &self.choices {
                if ui.button(&choice.display).clicked() {
                    picked = Some(choice.replacement.clone());
                }
            }
        });
        if let Some(replacement) = picked {
            let from = self.choices_from.min(self.typed.len());
            if self.typed.is_char_boundary(from) {
                let finished = format!("{}{replacement}", &self.typed[..from]);
                self.set_typed(ui, finished);
            }
            self.wants_focus = true;
        }
    }

    /// Changes what's typed and puts the text cursor at the end.
    fn set_typed(&mut self, ui: &egui::Ui, text: String) {
        self.typed = text;
        self.choices.clear();
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), input_id()) {
            let end = egui::text::CCursor::new(self.typed.chars().count());
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(end)));
            state.store(ui.ctx(), input_id());
        }
    }

    // ── Deciding and doing ──────────────────────────────────────────────────

    fn submit(&mut self, ctx: &egui::Context, line: &str, terminal: &mut TerminalView) {
        let line = line.trim();
        self.choices.clear();
        if line.is_empty() {
            terminal.focus();
            return;
        }
        self.history.push(line);
        self.last_line = line.to_string();
        // Names like "photos" mean the ones in the shell's folder, not Noob's.
        self.follow_shell_folder(terminal);
        let plan = bar::plan(line, &self.setup);
        self.answer.clear();
        self.about = None;
        match plan.ask {
            Some(question) => self.waiting = Some((question, plan.then)),
            None => self.carry_out(ctx, plan.then, terminal),
        }
    }

    fn carry_out(&mut self, ctx: &egui::Context, then: Then, terminal: &mut TerminalView) {
        match then {
            Then::Type { command, about } => {
                if let Some(why) = terminal.busy() {
                    self.answer = vec![Line::Problem(why.to_string())];
                    // Keep what you typed, so you can send it again once it's free.
                    self.typed = std::mem::take(&mut self.last_line);
                    self.wants_focus = true;
                    return;
                }
                terminal.type_into_prompt(&command);
                self.answer = vec![
                    Line::Done("Ready in the terminal. Press Enter there to run it.".to_string()),
                    Line::Real(command),
                ];
                self.about = Some(about);
            }
            Then::RunHere(command) => {
                let (answer, arrives) = mpsc::channel();
                let ctx = ctx.clone();
                std::thread::spawn(move || {
                    // Anything risky was already asked about, so any further
                    // question in the middle of the work is answered "no".
                    let _ = answer.send(engine::finish(&command, &mut |_| false));
                    ctx.request_repaint();
                });
                self.working = Some(arrives);
            }
            Then::Say(lines) => {
                self.answer = lines;
                self.wants_focus = true;
            }
        }
    }

    /// Picks up the answer from something Noob is doing itself, if it's ready.
    fn collect(&mut self) {
        let Some(arrives) = &self.working else {
            return;
        };
        match arrives.try_recv() {
            Ok(outcome) => {
                self.answer = outcome.lines;
                self.working = None;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.answer = vec![Line::Problem("That stopped unexpectedly.".to_string())];
                self.working = None;
            }
        }
    }

    /// "⚠ Move notes.txt to the Trash?" with Yes and No. Y and N work too, and
    /// Esc means No. While it's showing, keys don't reach the terminal.
    fn question_row(&mut self, ui: &mut egui::Ui, terminal: &mut TerminalView, theme: &Theme) {
        let Some((question, _)) = &self.waiting else {
            return;
        };
        let question = question.clone();
        let (yes_key, no_key) = ui.input_mut(|input| {
            let pressed = |input: &egui::InputState, wanted: Key| {
                input.events.iter().any(|event| {
                    matches!(event, Event::Key { key, pressed: true, .. } if *key == wanted)
                })
            };
            let yes = pressed(input, Key::Y);
            let no = pressed(input, Key::N) || pressed(input, Key::Escape);
            input
                .events
                .retain(|event| !matches!(event, Event::Text(_) | Event::Key { .. }));
            (yes, no)
        });
        let mut answer = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("⚠ {question}"))
                    .strong()
                    .color(theme.ansi[3]),
            );
            if ui.add(primary_button("Yes", theme)).clicked() || yes_key {
                answer = Some(true);
            }
            if ui.button("No").clicked() || no_key {
                answer = Some(false);
            }
            ui.label(
                RichText::new("or press Y / N")
                    .size(12.0)
                    .color(theme.ansi[8]),
            );
        });
        let Some(yes) = answer else {
            return;
        };
        let Some((_, then)) = self.waiting.take() else {
            return;
        };
        if yes {
            self.carry_out(ui.ctx(), then, terminal);
        } else {
            self.answer = engine::declined().lines;
            terminal.focus();
        }
    }

    // ── Showing Noob's answer ───────────────────────────────────────────────

    fn answer_area(&mut self, ui: &mut egui::Ui, theme: &Theme) {
        let mut close = false;
        let text_width = ui.available_width() - 28.0;
        ui.horizontal_top(|ui| {
            // One line under the other, scrolling if there are many (like `help`).
            ui.vertical(|ui| {
                ui.set_width(text_width);
                egui::ScrollArea::vertical()
                    .max_height(MAX_ANSWER_HEIGHT)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for line in &self.answer {
                            show_line(ui, line, theme);
                        }
                        if let Some(about) = &self.about {
                            ui.label(RichText::new(about).color(theme.ansi[8]));
                        }
                    });
            });
            close = ui
                .add(egui::Button::new(RichText::new("×").color(theme.ansi[8])).frame(false))
                .on_hover_text("Hide this")
                .clicked();
        });
        ui.add_space(6.0);
        if close {
            self.answer.clear();
            self.about = None;
        }
    }
}

fn show_line(ui: &mut egui::Ui, line: &Line, theme: &Theme) {
    let text = match line {
        Line::Done(message) => RichText::new(format!("✓ {message}")).color(theme.ansi[2]),
        Line::Problem(message) => RichText::new(format!("✗ {message}")).color(theme.ansi[1]),
        Line::Hint(message) => RichText::new(message.as_str()).color(theme.ansi[3]),
        // Noob's own answers (like `help`) line up in columns, so they keep
        // the terminal's letters, where every letter is the same width.
        Line::Show(text) => RichText::new(text.as_str())
            .monospace()
            .size(13.0)
            .color(theme.fg),
        // The real command, shown the way it looks in the terminal.
        Line::Real(real) => RichText::new(format!("  {real}"))
            .monospace()
            .size(INPUT_TEXT_SIZE)
            .color(theme.cursor),
    };
    ui.label(text);
}

fn primary_button<'a>(label: &'a str, theme: &Theme) -> egui::Button<'a> {
    egui::Button::new(RichText::new(label).strong().color(theme.bg))
        .fill(theme.cursor)
        .corner_radius(5.0)
        .min_size(egui::vec2(56.0, 0.0))
}

/// Cmd+K on a Mac, Ctrl+K on Windows. Linux shells use Ctrl+K themselves
/// ("delete to the end of the line"), so there it's Ctrl+Shift+K.
fn shortcut_pressed(ui: &egui::Ui) -> bool {
    let modifiers = if cfg!(target_os = "linux") {
        Modifiers::COMMAND | Modifiers::SHIFT
    } else {
        Modifiers::COMMAND
    };
    ui.input_mut(|input| input.consume_key(modifiers, Key::K))
}

fn shortcut_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘K"
    } else if cfg!(target_os = "linux") {
        "Ctrl+Shift+K"
    } else {
        "Ctrl+K"
    }
}

/// Lines typed into the bar before, for the up and down arrows.
#[derive(Debug, Default)]
struct History {
    lines: Vec<String>,
    /// Which line the arrows are showing (`None`: a fresh, empty box).
    at: Option<usize>,
}

impl History {
    fn push(&mut self, line: &str) {
        if self.lines.last().map(String::as_str) != Some(line) {
            self.lines.push(line.to_string());
        }
        self.at = None;
    }

    /// One line further back (stays on the oldest).
    fn older(&mut self) -> Option<String> {
        let last = self.lines.len().checked_sub(1)?;
        let at = self.at.map_or(last, |at| at.saturating_sub(1));
        self.at = Some(at);
        self.lines.get(at).cloned()
    }

    /// One line forward; past the newest comes an empty box again.
    fn newer(&mut self) -> Option<String> {
        let at = self.at?;
        if at + 1 < self.lines.len() {
            self.at = Some(at + 1);
            self.lines.get(at + 1).cloned()
        } else {
            self.at = None;
            Some(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(lines: &[&str]) -> History {
        let mut history = History::default();
        for line in lines {
            history.push(line);
        }
        history
    }

    #[test]
    fn up_goes_back_through_earlier_lines_and_stops_at_the_oldest() {
        let mut history = history(&["one", "two"]);
        assert_eq!(history.older().as_deref(), Some("two"));
        assert_eq!(history.older().as_deref(), Some("one"));
        assert_eq!(history.older().as_deref(), Some("one"));
    }

    #[test]
    fn down_comes_forward_and_ends_on_an_empty_box() {
        let mut history = history(&["one", "two"]);
        history.older();
        history.older();
        assert_eq!(history.newer().as_deref(), Some("two"));
        assert_eq!(history.newer().as_deref(), Some(""));
        assert_eq!(history.newer(), None, "already at the empty box");
    }

    #[test]
    fn nothing_typed_yet_means_nothing_to_go_back_to() {
        assert_eq!(History::default().older(), None);
    }

    #[test]
    fn the_same_line_twice_in_a_row_is_kept_once() {
        let mut history = history(&["ls", "ls"]);
        assert_eq!(history.older().as_deref(), Some("ls"));
        assert_eq!(history.older().as_deref(), Some("ls"));
        assert_eq!(history.lines.len(), 1);
    }
}
