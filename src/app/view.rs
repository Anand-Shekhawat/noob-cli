//! Drawing Noob's window: a top bar, the conversation, and the typing box.
//!
//! egui redraws the whole window many times a second. Each time, `ui` below describes
//! what should be on screen, and reports what was clicked or typed since last time.

use eframe::egui::{self, Color32, Key, Modifiers, RichText};
use noob_cli::chat::{Chat, Entry};
use noob_cli::complete::Choice;
use noob_cli::engine::Line;
use noob_cli::terminal::TerminalWindow;

const INPUT: &str = "noob-typing-box";
const TOP_BAR_HEIGHT: f32 = 40.0;
const BOTTOM_BAR_HEIGHT: f32 = 48.0;

/// The quick buttons in the top bar, and what each one types for you.
const BUTTONS: [(&str, &str); 4] = [
    ("Help", "help"),
    ("Home", "go home"),
    ("Back", "go back"),
    ("Files", "show files"),
];

pub struct NoobApp {
    chat: Chat,
    typed: String,
    /// Tab choices waiting to be picked, and where in `typed` they go.
    choices: Vec<Choice>,
    choices_start: usize,
}

impl NoobApp {
    pub fn new() -> Self {
        #[allow(unused_mut)] // only changed when checking how the window looks
        let mut app = NoobApp {
            chat: Chat::new(Box::new(TerminalWindow)),
            typed: String::new(),
            choices: Vec::new(),
            choices_start: 0,
        };
        // For developers checking how the window looks: NOOB_DEMO="where am i;show files"
        // types those lines at startup. Only in the `screenshot` build, never for users.
        #[cfg(feature = "screenshot")]
        for line in std::env::var("NOOB_DEMO").unwrap_or_default().split(';') {
            app.send(line);
        }
        app
    }

    fn send(&mut self, line: &str) {
        self.chat.send(line);
        self.typed.clear();
        self.choices.clear();
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        // Buttons first, from the right edge; the folder's name gets the space that's left.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for (label, line) in BUTTONS {
                if ui.button(label).clicked() && self.chat.question().is_none() {
                    self.send(line);
                }
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let name = RichText::new(format!("📁 {}", self.chat.folder_name()))
                    .strong()
                    .size(15.0);
                // Too long to fit? It's cut short with "…". Hovering shows the full path.
                ui.add(egui::Label::new(name).truncate())
                    .on_hover_text(self.chat.folder());
            });
        });
        ui.add_space(4.0);
    }

    fn bottom_bar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        if let Some(question) = self.chat.question().map(str::to_string) {
            self.question_bar(ui, &question);
        } else {
            if !self.choices.is_empty() {
                self.choice_buttons(ui);
            }
            self.typing_box(ui);
        }
        ui.add_space(6.0);
    }

    /// "Are you sure?" with Yes and No buttons. Y and N on the keyboard work too.
    fn question_bar(&mut self, ui: &mut egui::Ui, question: &str) {
        let colours = Colours::for_ui(ui);
        ui.label(
            RichText::new(format!("⚠ {question}"))
                .color(colours.hint)
                .strong(),
        );
        ui.horizontal(|ui| {
            let (yes_key, no_key) = ui.input_mut(|input| {
                (
                    input.consume_key(Modifiers::NONE, Key::Y),
                    input.consume_key(Modifiers::NONE, Key::N)
                        || input.consume_key(Modifiers::NONE, Key::Escape),
                )
            });
            if ui.button("Yes").clicked() || yes_key {
                self.chat.answer(true);
            } else if ui.button("No").clicked() || no_key {
                self.chat.answer(false);
            }
        });
    }

    fn typing_box(&mut self, ui: &mut egui::Ui) {
        let id = egui::Id::unique(INPUT);
        if ui.memory(|memory| memory.has_focus(id)) {
            self.special_keys(ui);
        }
        let response = ui.add(
            egui::TextEdit::singleline(&mut self.typed)
                .id(id)
                .hint_text("Type here, like: make folder photos")
                .font(egui::TextStyle::Monospace)
                .desired_width(f32::INFINITY)
                .lock_focus(true), // Tab finishes words instead of leaving the box
        );
        if response.changed() {
            self.choices.clear();
        }
        if response.lost_focus() && ui.input(|input| input.key_pressed(Key::Enter)) {
            let line = std::mem::take(&mut self.typed);
            self.send(&line);
        }
        // Keep the typing box ready, unless something else (like a button) has focus.
        if !response.has_focus() && ui.memory(|memory| memory.focused().is_none()) {
            response.request_focus();
        }
    }

    /// Up/down for earlier commands, and Tab to finish words. Taken before the typing
    /// box sees them, so it doesn't move the cursor instead.
    fn special_keys(&mut self, ui: &mut egui::Ui) {
        let (up, down, tab) = ui.input_mut(|input| {
            (
                input.consume_key(Modifiers::NONE, Key::ArrowUp),
                input.consume_key(Modifiers::NONE, Key::ArrowDown),
                input.consume_key(Modifiers::NONE, Key::Tab),
            )
        });
        if up && let Some(line) = self.chat.older() {
            self.set_typed(ui, line);
        }
        if down && let Some(line) = self.chat.newer() {
            self.set_typed(ui, line);
        }
        if tab {
            self.finish_word(ui);
        }
    }

    fn finish_word(&mut self, ui: &egui::Ui) {
        let (start, choices) = self.chat.complete(&self.typed);
        match choices.as_slice() {
            [] => {}
            [only] => {
                let finished = self.with_choice(start, &only.replacement);
                self.set_typed(ui, finished);
                self.choices.clear();
            }
            _ => {
                self.choices = choices;
                self.choices_start = start;
            }
        }
    }

    fn choice_buttons(&mut self, ui: &mut egui::Ui) {
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Pick one:").weak());
            for choice in &self.choices {
                if ui.button(&choice.display).clicked() {
                    picked = Some(choice.replacement.clone());
                }
            }
        });
        if let Some(replacement) = picked {
            let finished = self.with_choice(self.choices_start, &replacement);
            self.set_typed(ui, finished);
            self.choices.clear();
        }
    }

    /// What's typed up to `start`, followed by the chosen replacement.
    fn with_choice(&self, start: usize, replacement: &str) -> String {
        let before = self.typed.get(..start).unwrap_or(&self.typed);
        format!("{before}{replacement}")
    }

    /// Changes what's typed, and puts the text cursor at the end so you can keep typing.
    fn set_typed(&mut self, ui: &egui::Ui, text: String) {
        self.typed = text;
        let id = egui::Id::unique(INPUT);
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
            let end = egui::text::CCursor::new(self.typed.chars().count());
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::one(end)));
            state.store(ui.ctx(), id);
        }
    }

    fn conversation(&self, ui: &mut egui::Ui) {
        let colours = Colours::for_ui(ui);
        egui::ScrollArea::vertical()
            .stick_to_bottom(true) // new messages scroll into view
            .auto_shrink(false)
            .show(ui, |ui| {
                for entry in self.chat.entries() {
                    show_entry(ui, entry, &colours);
                }
            });
    }
}

impl eframe::App for NoobApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.chat.wants_to_quit() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // Fixed sizes, so the bars look right from the very first moment.
        egui::Panel::top("top-bar")
            .exact_size(TOP_BAR_HEIGHT)
            .show(ui, |ui| self.top_bar(ui));
        egui::Panel::bottom("bottom-bar")
            .min_size(BOTTOM_BAR_HEIGHT)
            .show(ui, |ui| self.bottom_bar(ui));
        egui::CentralPanel::default().show(ui, |ui| self.conversation(ui));
    }
}

fn show_entry(ui: &mut egui::Ui, entry: &Entry, colours: &Colours) {
    let text = match entry {
        Entry::You(typed) => {
            ui.add_space(8.0);
            RichText::new(format!("› {typed}"))
                .strong()
                .color(colours.you)
        }
        Entry::Noob(Line::Done(message)) => {
            RichText::new(format!("✓ {message}")).color(colours.done)
        }
        Entry::Noob(Line::Show(text)) => RichText::new(text.as_str()),
        Entry::Noob(Line::Problem(message)) => {
            RichText::new(format!("✗ {message}")).color(colours.problem)
        }
        Entry::Noob(Line::Hint(message)) => RichText::new(message.as_str()).color(colours.hint),
        Entry::Noob(Line::Real(real)) => RichText::new(format!("real command: {real}"))
            .small()
            .weak(),
    };
    ui.label(text.monospace());
}

/// Colours that are easy to read on both light and dark backgrounds.
struct Colours {
    you: Color32,
    done: Color32,
    problem: Color32,
    hint: Color32,
}

impl Colours {
    fn for_ui(ui: &egui::Ui) -> Colours {
        if ui.visuals().dark_mode {
            Colours {
                you: Color32::from_rgb(120, 190, 255),
                done: Color32::from_rgb(120, 210, 130),
                problem: Color32::from_rgb(255, 120, 110),
                hint: Color32::from_rgb(240, 200, 90),
            }
        } else {
            Colours {
                you: Color32::from_rgb(20, 90, 180),
                done: Color32::from_rgb(20, 120, 40),
                problem: Color32::from_rgb(190, 30, 30),
                hint: Color32::from_rgb(150, 100, 0),
            }
        }
    }
}
