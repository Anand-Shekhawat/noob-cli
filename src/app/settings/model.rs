//! Your settings, and how they're written in the settings file.
//!
//! The file is plain text you could edit yourself:
//!
//! ```text
//! # Noob's settings.
//! font_size = 16
//! theme = dark
//! ```
//!
//! Reading it never fails: a missing file, an unknown line or a bad value just
//! means the normal setting for that line.

/// The normal letter size in the terminal, and how small or big it may get.
pub const NORMAL_FONT_SIZE: f32 = 14.0;
pub const SMALLEST_FONT_SIZE: f32 = 10.0;
pub const BIGGEST_FONT_SIZE: f32 = 28.0;

/// Which colours Noob uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// Dark or light, whichever the computer is set to.
    #[default]
    System,
    Dark,
    Light,
    /// Bright text on black, for easier reading.
    HighContrast,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 4] = [
        ThemeChoice::System,
        ThemeChoice::Dark,
        ThemeChoice::Light,
        ThemeChoice::HighContrast,
    ];

    /// The word used for it in the settings file.
    pub fn word(self) -> &'static str {
        match self {
            ThemeChoice::System => "system",
            ThemeChoice::Dark => "dark",
            ThemeChoice::Light => "light",
            ThemeChoice::HighContrast => "high-contrast",
        }
    }

    /// The name shown in the Settings panel.
    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::System => "Follow my computer",
            ThemeChoice::Dark => "Dark",
            ThemeChoice::Light => "Light",
            ThemeChoice::HighContrast => "High contrast",
        }
    }

    fn from_word(word: &str) -> Option<ThemeChoice> {
        ThemeChoice::ALL
            .into_iter()
            .find(|choice| choice.word() == word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub font_size: f32,
    pub theme: ThemeChoice,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            font_size: NORMAL_FONT_SIZE,
            theme: ThemeChoice::System,
        }
    }
}

/// How much one Cmd+ or Cmd− changes the letter size.
const FONT_STEP: f32 = 1.0;

impl Settings {
    /// Settings read from the file's text. Anything not understood keeps
    /// its normal value.
    pub fn from_text(text: &str) -> Settings {
        let mut settings = Settings::default();
        // Windows Notepad may put an invisible "BOM" mark at the very start.
        let text = text.trim_start_matches('\u{feff}');
        for line in text.lines().map(str::trim) {
            if line.starts_with('#') {
                continue;
            }
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            match (name.trim(), value.trim()) {
                ("font_size", size) => {
                    if let Some(size) = size.parse::<f32>().ok().filter(|size| size.is_finite()) {
                        settings = settings.with_font_size(size);
                    }
                }
                ("theme", word) => {
                    if let Some(theme) = ThemeChoice::from_word(word) {
                        settings.theme = theme;
                    }
                }
                _ => {} // a line from a newer Noob, or a typo: skip it
            }
        }
        settings
    }

    /// The text to write into the settings file.
    pub fn to_text(self) -> String {
        format!(
            "# Noob's settings. Change them in Noob (Cmd+, or Ctrl+,) or here.\n\
             # theme can be: system, dark, light or high-contrast.\n\
             font_size = {}\n\
             theme = {}\n",
            self.font_size,
            self.theme.word()
        )
    }

    /// One size bigger (Cmd+), up to the biggest.
    pub fn bigger(self) -> Settings {
        self.with_font_size(self.font_size + FONT_STEP)
    }

    /// One size smaller (Cmd−), down to the smallest.
    pub fn smaller(self) -> Settings {
        self.with_font_size(self.font_size - FONT_STEP)
    }

    /// Back to the normal size (Cmd+0).
    pub fn normal_size(self) -> Settings {
        self.with_font_size(NORMAL_FONT_SIZE)
    }

    /// The given size, kept between the smallest and biggest.
    pub fn with_font_size(self, size: f32) -> Settings {
        Settings {
            font_size: size.round().clamp(SMALLEST_FONT_SIZE, BIGGEST_FONT_SIZE),
            ..self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn written_settings_read_back_the_same() {
        let settings = Settings {
            font_size: 17.0,
            theme: ThemeChoice::HighContrast,
        };
        assert_eq!(Settings::from_text(&settings.to_text()), settings);
    }

    #[test]
    fn the_file_is_readable_by_a_person() {
        let text = Settings {
            font_size: 16.0,
            theme: ThemeChoice::Dark,
        }
        .to_text();
        assert!(text.contains("font_size = 16"), "{text}");
        assert!(text.contains("theme = dark"), "{text}");
        assert!(text.starts_with('#'), "a comment explains the file");
    }

    #[test]
    fn an_empty_or_missing_file_means_normal_settings() {
        assert_eq!(Settings::from_text(""), Settings::default());
    }

    #[test]
    fn spaces_comments_and_unknown_lines_are_fine() {
        let text = "# hi\n\n  theme=light  \nfavourite_colour = blue\nfont_size =  12\n";
        assert_eq!(
            Settings::from_text(text),
            Settings {
                font_size: 12.0,
                theme: ThemeChoice::Light,
            }
        );
    }

    #[test]
    fn a_file_saved_by_windows_notepad_still_reads() {
        // Notepad may start the file with an invisible mark (a "BOM") and end
        // lines with \r\n.
        let text = "\u{feff}font_size = 20\r\ntheme = light\r\n";
        assert_eq!(
            Settings::from_text(text),
            Settings {
                font_size: 20.0,
                theme: ThemeChoice::Light,
            }
        );
    }

    #[test]
    fn a_bad_value_keeps_the_normal_one_for_that_line_only() {
        let text = "font_size = huge\ntheme = dark\n";
        assert_eq!(
            Settings::from_text(text),
            Settings {
                font_size: NORMAL_FONT_SIZE,
                theme: ThemeChoice::Dark,
            }
        );
        assert_eq!(
            Settings::from_text("theme = purple").theme,
            ThemeChoice::System
        );
    }

    #[test]
    fn a_font_size_out_of_range_is_brought_back_in() {
        assert_eq!(
            Settings::from_text("font_size = 500").font_size,
            BIGGEST_FONT_SIZE
        );
        assert_eq!(
            Settings::from_text("font_size = -3").font_size,
            SMALLEST_FONT_SIZE
        );
        assert_eq!(
            Settings::from_text("font_size = NaN").font_size,
            NORMAL_FONT_SIZE,
            "not a number at all"
        );
    }

    #[test]
    fn bigger_and_smaller_step_by_one_and_stop_at_the_ends() {
        let normal = Settings::default();
        assert_eq!(normal.bigger().font_size, NORMAL_FONT_SIZE + 1.0);
        assert_eq!(normal.smaller().font_size, NORMAL_FONT_SIZE - 1.0);
        let biggest = normal.with_font_size(BIGGEST_FONT_SIZE);
        assert_eq!(biggest.bigger().font_size, BIGGEST_FONT_SIZE);
        let smallest = normal.with_font_size(SMALLEST_FONT_SIZE);
        assert_eq!(smallest.smaller().font_size, SMALLEST_FONT_SIZE);
        assert_eq!(biggest.normal_size().font_size, NORMAL_FONT_SIZE);
    }

    #[test]
    fn every_theme_has_its_own_word() {
        for choice in ThemeChoice::ALL {
            assert_eq!(ThemeChoice::from_word(choice.word()), Some(choice));
        }
    }
}
