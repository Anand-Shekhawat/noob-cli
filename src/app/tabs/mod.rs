//! Tabs: several shells in one window.
//!
//! Each tab is its own `TerminalView` (its own shell, history and folder).
//! The tab strip only shows once there are two or more, like in Terminal and
//! iTerm. Background tabs keep reading their shell's output, and get a dot
//! when something new appears there.
//!
//!   - `list`: the tabs in order, and which one is active (pure, tested).
//!   - `keys`: which key presses are tab shortcuts (pure, tested).
//!   - `title`: the short name shown on a tab (pure, tested).

mod keys;
mod list;
mod title;

use eframe::egui::{self, Color32, CornerRadius, Margin, RichText, Sense};

use crate::terminal::{TerminalView, Theme};
use keys::{TabAction, tab_action};
use list::TabList;
use title::short_title;

/// The widest a tab gets, so a few tabs don't stretch across the whole window.
const MAX_TAB_WIDTH: f32 = 200.0;

struct Tab {
    terminal: TerminalView,
    /// Something new was printed while you were looking at another tab.
    unseen: bool,
}

impl Tab {
    fn new(terminal: TerminalView) -> Tab {
        Tab {
            terminal,
            unseen: false,
        }
    }
}

pub struct Tabs {
    list: TabList<Tab>,
    /// A tab waiting for "Close it anyway?" because a program runs in it.
    confirm_close: Option<usize>,
}

impl Tabs {
    pub fn new() -> Self {
        Tabs {
            list: TabList::new(Tab::new(TerminalView::new())),
            confirm_close: None,
        }
    }

    /// The terminal you're looking at.
    pub fn active(&mut self) -> &mut TerminalView {
        &mut self.list.active_mut().terminal
    }

    /// The title of the terminal you're looking at (for the window's title).
    pub fn active_title(&self) -> Option<&str> {
        self.list.active().terminal.title()
    }

    /// Reads tab shortcuts before anything else sees those keys. Returns true
    /// when the window should close (you closed the last tab).
    pub fn shortcuts(&mut self, ui: &mut egui::Ui) -> bool {
        let on_mac = cfg!(target_os = "macos");
        let actions: Vec<TabAction> = ui.input_mut(|input| {
            let mut actions = Vec::new();
            input.events.retain(|event| match event {
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => match tab_action(*key, *modifiers, on_mac) {
                    Some(action) => {
                        if *pressed {
                            actions.push(action);
                        }
                        false // a tab shortcut never reaches the shell
                    }
                    None => true,
                },
                _ => true,
            });
            actions
        });
        let mut close_window = false;
        for action in actions {
            close_window |= self.carry_out(action);
        }
        close_window
    }

    /// True when the window should close.
    fn carry_out(&mut self, action: TabAction) -> bool {
        match action {
            TabAction::New => self.open_tab(),
            TabAction::Close => return self.ask_to_close(self.list.active_index()),
            TabAction::Next => self.list.next(),
            TabAction::Previous => self.list.previous(),
            TabAction::Select(index) => self.list.select(index),
            TabAction::Last => self.list.select(self.list.len() - 1),
        }
        self.list.active_mut().unseen = false;
        false
    }

    /// A new tab, in the folder the current tab's shell is in.
    fn open_tab(&mut self) {
        let folder = self.list.active().terminal.shell_folder();
        self.list.add(Tab::new(TerminalView::starting_in(folder)));
    }

    /// Closes a tab, asking first if a program is still running in it.
    /// True when that was the last tab, so the window should close.
    fn ask_to_close(&mut self, index: usize) -> bool {
        let running = self
            .list
            .iter_mut()
            .any(|(at, tab)| at == index && tab.terminal.is_running_program());
        if running {
            self.confirm_close = Some(index);
            return false;
        }
        self.close_now(index)
    }

    fn close_now(&mut self, index: usize) -> bool {
        self.confirm_close = None;
        if self.list.len() == 1 {
            return true;
        }
        self.list.close(index);
        self.list.active_mut().unseen = false;
        false
    }

    /// Lets the tabs you're not looking at take in their output.
    pub fn keep_up_in_background(&mut self) {
        let active = self.list.active_index();
        for (index, tab) in self.list.iter_mut() {
            if index != active && tab.terminal.keep_up() {
                tab.unseen = true;
            }
        }
    }

    /// Whether the tab strip should be on screen.
    pub fn strip_wanted(&self) -> bool {
        self.list.len() > 1 || self.confirm_close.is_some()
    }

    /// The tab strip (or the "close it anyway?" question). True when the
    /// window should close.
    pub fn strip(&mut self, ui: &mut egui::Ui, theme: &Theme) -> bool {
        if let Some(index) = self.confirm_close {
            return self.close_question(ui, theme, index);
        }
        let mut clicked = None;
        let mut close = None;
        let mut add = false;
        let active = self.list.active_index();
        let count = self.list.len();
        let width = ((ui.available_width() - 40.0) / count as f32).min(MAX_TAB_WIDTH);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (index, tab) in self.list.iter_mut() {
                let name = short_title(tab.terminal.title());
                match tab_widget(ui, theme, &name, index == active, tab.unseen, width) {
                    TabClick::Select => clicked = Some(index),
                    TabClick::Close => close = Some(index),
                    TabClick::Nothing => {}
                }
            }
            let plus =
                egui::Button::new(RichText::new("+").size(16.0).color(theme.ansi[8])).frame(false);
            add = ui
                .add(plus)
                .on_hover_text(format!("New tab ({})", new_tab_shortcut()))
                .clicked();
        });
        if let Some(index) = clicked {
            self.list.select(index);
            self.list.active_mut().unseen = false;
        }
        if add {
            self.open_tab();
        }
        match close {
            Some(index) => self.ask_to_close(index),
            None => false,
        }
    }

    fn close_question(&mut self, ui: &mut egui::Ui, theme: &Theme, index: usize) -> bool {
        let (yes_key, no_key) = ui.input_mut(|input| {
            (
                input.consume_key(egui::Modifiers::NONE, egui::Key::Y),
                input.consume_key(egui::Modifiers::NONE, egui::Key::N)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
            )
        });
        let mut answer = None;
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("⚠ A program is still running in this tab. Close it anyway?")
                    .strong()
                    .color(theme.ansi[3]),
            );
            let close = egui::Button::new(RichText::new("Close").strong().color(theme.bg))
                .fill(theme.cursor)
                .corner_radius(5.0);
            if ui.add(close).clicked() || yes_key {
                answer = Some(true);
            }
            if ui.button("Keep it").clicked() || no_key {
                answer = Some(false);
            }
        });
        match answer {
            Some(true) => self.close_now(index),
            Some(false) => {
                self.confirm_close = None;
                false
            }
            None => false,
        }
    }
}

enum TabClick {
    Nothing,
    Select,
    Close,
}

/// One tab: its name (with a dot if there's news), and a × to close it.
fn tab_widget(
    ui: &mut egui::Ui,
    theme: &Theme,
    name: &str,
    active: bool,
    unseen: bool,
    width: f32,
) -> TabClick {
    // The active tab takes the terminal's colour, so it looks joined to it.
    let (fill, text) = if active {
        (theme.bg, theme.fg)
    } else {
        (Color32::TRANSPARENT, theme.ansi[8])
    };
    let mut click = TabClick::Nothing;
    egui::Frame::new()
        .fill(fill)
        .corner_radius(CornerRadius {
            nw: 6,
            ne: 6,
            sw: 0,
            se: 0,
        })
        .inner_margin(Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.set_width(width - 20.0);
            ui.horizontal(|ui| {
                let close_room = 18.0;
                if unseen {
                    // A small dot in the accent colour: something new in here.
                    ui.label(RichText::new("•").strong().color(theme.cursor))
                        .on_hover_text("Something new was printed in this tab");
                }
                let name = egui::Label::new(RichText::new(name).color(text))
                    .truncate()
                    .sense(Sense::click());
                ui.scope(|ui| {
                    ui.set_max_width(ui.available_width() - close_room);
                    if ui.add(name).clicked() {
                        click = TabClick::Select;
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close =
                        egui::Button::new(RichText::new("×").color(theme.ansi[8])).frame(false);
                    if ui
                        .add(close)
                        .on_hover_text(format!("Close tab ({})", close_tab_shortcut()))
                        .clicked()
                    {
                        click = TabClick::Close;
                    }
                });
            });
        });
    click
}

fn new_tab_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘T"
    } else {
        "Ctrl+Shift+T"
    }
}

fn close_tab_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘W"
    } else {
        "Ctrl+Shift+W"
    }
}
