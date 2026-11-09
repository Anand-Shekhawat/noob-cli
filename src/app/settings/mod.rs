//! Settings: letter size and colours, remembered in a small text file.
//!
//!   - `model`: the settings, and the file's text (pure, tested).
//!   - `file`: where the file lives, loading and saving it.
//!   - `keys`: Cmd+ / Cmd− / Cmd+0 / Cmd+, (pure, tested).
//!
//! This file holds them while Noob runs, draws the Settings panel, and saves
//! after every change.

mod file;
mod keys;
mod model;

use std::path::PathBuf;

use eframe::egui::{self, RichText};

use crate::terminal::Theme;
use keys::{SettingsAction, settings_action};
use model::{BIGGEST_FONT_SIZE, SMALLEST_FONT_SIZE, Settings, ThemeChoice};

pub struct SettingsState {
    settings: Settings,
    /// Where they're saved (`None` if there's no home folder to save in).
    path: Option<PathBuf>,
    panel_open: bool,
    /// Why the last save failed, shown in the panel.
    save_problem: Option<String>,
}

impl SettingsState {
    /// Loads your settings (or the normal ones), and turns off egui's own
    /// Cmd+ / Cmd− zoom, which would enlarge the whole window instead.
    pub fn load(ctx: &egui::Context) -> Self {
        ctx.options_mut(|options| options.zoom_with_keyboard = false);
        let path = file::settings_path(file::System::current(), |name| std::env::var(name).ok());
        // Developer screenshots use their own settings file, never yours.
        #[cfg(feature = "screenshot")]
        let path = std::env::var_os("NOOB_SETTINGS_FILE")
            .map(PathBuf::from)
            .or(path);
        let settings = path.as_deref().map(file::load).unwrap_or_default();
        ctx.set_theme(egui_theme(settings.theme));
        SettingsState {
            settings,
            path,
            panel_open: false,
            save_problem: None,
        }
    }

    /// While the panel is open, keys belong to it, not to the terminal.
    pub fn panel_open(&self) -> bool {
        self.panel_open
    }

    pub fn font_size(&self) -> f32 {
        self.settings.font_size
    }

    /// The colours to draw with, now.
    pub fn theme(&self, ui: &egui::Ui) -> Theme {
        theme_for(self.settings.theme, ui.visuals().dark_mode)
    }

    /// Reads the settings shortcuts before anything else sees those keys.
    pub fn shortcuts(&mut self, ui: &mut egui::Ui) {
        // Esc closes the panel, and is taken here so the shell never gets it.
        if self.panel_open
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.panel_open = false;
        }
        let on_mac = cfg!(target_os = "macos");
        let actions: Vec<SettingsAction> = ui.input_mut(|input| {
            let mut actions = Vec::new();
            input.events.retain(|event| match event {
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => match settings_action(*key, *modifiers, on_mac) {
                    Some(action) => {
                        if *pressed {
                            actions.push(action);
                        }
                        false // a settings shortcut never reaches the shell
                    }
                    None => true,
                },
                _ => true,
            });
            actions
        });
        for action in actions {
            let settings = self.settings;
            match action {
                SettingsAction::Bigger => self.change(ui.ctx(), settings.bigger()),
                SettingsAction::Smaller => self.change(ui.ctx(), settings.smaller()),
                SettingsAction::NormalSize => self.change(ui.ctx(), settings.normal_size()),
                SettingsAction::OpenPanel => self.panel_open = !self.panel_open,
            }
        }
    }

    /// Uses and saves new settings (only if something really changed).
    fn change(&mut self, ctx: &egui::Context, settings: Settings) {
        if settings == self.settings {
            return;
        }
        if settings.theme != self.settings.theme {
            ctx.set_theme(egui_theme(settings.theme));
        }
        self.settings = settings;
        self.save_problem = match &self.path {
            Some(path) => file::save(path, &settings)
                .err()
                .map(|error| format!("Couldn't save your settings: {error}")),
            None => Some("There's no home folder to save your settings in.".to_string()),
        };
    }

    /// The Settings panel, when it's open.
    pub fn panel(&mut self, ctx: &egui::Context, theme: &Theme) {
        if !self.panel_open {
            return;
        }
        let mut settings = self.settings;
        let mut open = true;
        egui::Window::new("Settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 10.0;
                ui.label(RichText::new("Letter size").strong());
                ui.add(
                    egui::Slider::new(
                        &mut settings.font_size,
                        SMALLEST_FONT_SIZE..=BIGGEST_FONT_SIZE,
                    )
                    .step_by(1.0)
                    .fixed_decimals(0)
                    .suffix(" pt"),
                );
                ui.label(
                    RichText::new(format!(
                        "Shortcuts: {0}+ bigger, {0}− smaller, {0}0 normal",
                        cmd()
                    ))
                    .small()
                    .color(theme.ansi[8]),
                );
                ui.separator();
                ui.label(RichText::new("Colours").strong());
                for choice in ThemeChoice::ALL {
                    ui.radio_value(&mut settings.theme, choice, choice.label());
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Reset to normal").clicked() {
                        settings = Settings::default();
                    }
                    ui.label(
                        RichText::new(format!("Open this any time with {}", panel_shortcut()))
                            .small()
                            .color(theme.ansi[8]),
                    );
                });
                if let Some(problem) = &self.save_problem {
                    ui.label(RichText::new(format!("✗ {problem}")).color(theme.ansi[1]));
                }
            });
        // The slider can land between whole sizes; keep them whole and in range.
        self.change(ctx, settings.with_font_size(settings.font_size));
        if !open {
            self.panel_open = false;
        }
    }
}

/// The terminal's colours for a theme choice. `dark_mode` is what the computer
/// (or egui) is using, for "follow my computer".
fn theme_for(choice: ThemeChoice, dark_mode: bool) -> Theme {
    match choice {
        ThemeChoice::HighContrast => Theme::high_contrast(),
        ThemeChoice::Dark => Theme::dark(),
        ThemeChoice::Light => Theme::light(),
        ThemeChoice::System => Theme::for_dark_mode(dark_mode),
    }
}

/// egui's own look (buttons, panels) to go with a theme choice.
fn egui_theme(choice: ThemeChoice) -> egui::ThemePreference {
    match choice {
        ThemeChoice::System => egui::ThemePreference::System,
        ThemeChoice::Dark | ThemeChoice::HighContrast => egui::ThemePreference::Dark,
        ThemeChoice::Light => egui::ThemePreference::Light,
    }
}

fn cmd() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl+"
    }
}

fn panel_shortcut() -> &'static str {
    if cfg!(target_os = "macos") {
        "⌘,"
    } else {
        "Ctrl+,"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn following_the_computer_uses_its_dark_or_light_look() {
        assert_eq!(theme_for(ThemeChoice::System, true), Theme::dark());
        assert_eq!(theme_for(ThemeChoice::System, false), Theme::light());
    }

    #[test]
    fn a_chosen_theme_ignores_the_computer() {
        assert_eq!(theme_for(ThemeChoice::Dark, false), Theme::dark());
        assert_eq!(theme_for(ThemeChoice::Light, true), Theme::light());
        assert_eq!(
            theme_for(ThemeChoice::HighContrast, false),
            Theme::high_contrast()
        );
    }

    #[test]
    fn high_contrast_gets_egui_s_dark_look_around_it() {
        assert_eq!(
            egui_theme(ThemeChoice::HighContrast),
            egui::ThemePreference::Dark
        );
        assert_eq!(
            egui_theme(ThemeChoice::System),
            egui::ThemePreference::System
        );
    }
}
