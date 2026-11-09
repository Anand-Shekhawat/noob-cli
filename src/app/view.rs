//! Drawing Noob's window: your real shell fills it (see `terminal`), with the
//! Noob bar along the bottom (see `bar`) for asking in plain English.
//!
//! egui redraws the whole window many times a second. Each time, `ui` below describes
//! what should be on screen, and reports what was clicked or typed since last time.

use eframe::egui;

use crate::bar::NoobBar;
use crate::terminal::{TerminalView, Theme};

/// Space around the Noob bar's contents.
const BAR_MARGIN: egui::Margin = egui::Margin {
    left: 14,
    right: 10,
    top: 8,
    bottom: 10,
};

pub struct NoobApp {
    terminal: TerminalView,
    bar: NoobBar,
    /// The window title last set, so it's only changed when the shell's changes.
    title: String,
}

impl NoobApp {
    pub fn new() -> Self {
        NoobApp {
            terminal: TerminalView::new(),
            bar: NoobBar::new(),
            title: String::new(),
        }
    }

    /// The shell's own title (often "you@computer:~/folder") in the window's
    /// title bar, so you can always see where you are.
    fn update_title(&mut self, ctx: &egui::Context) {
        let title = match self.terminal.title() {
            Some(shell) => format!("{shell} — Noob"),
            None => "Noob".to_string(),
        };
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

impl eframe::App for NoobApp {
    #[cfg(feature = "screenshot")]
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        crate::devshot::press_keys(raw_input);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(feature = "screenshot")]
        crate::devshot::maybe_take(ui.ctx());
        let theme = Theme::for_dark_mode(ui.visuals().dark_mode);

        // The bar goes first, so its shortcut keys are taken before the terminal
        // sees them; it's drawn at the bottom either way.
        let bar_frame = egui::Frame::side_top_panel(ui.style())
            .fill(theme.bar)
            .inner_margin(BAR_MARGIN);
        egui::Panel::bottom("noob-bar")
            .frame(bar_frame)
            .show(ui, |ui| self.bar.ui(ui, &mut self.terminal, &theme));
        // No margin: the terminal paints its own background to the edges.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.terminal.ui(ui));
        self.update_title(ui.ctx());
    }
}
