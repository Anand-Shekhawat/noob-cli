//! Where the settings file lives, and reading and saving it.
//!
//!   - Mac:     ~/Library/Application Support/Noob/settings.txt
//!   - Windows: %APPDATA%\Noob\settings.txt
//!   - Linux:   ~/.config/noob/settings.txt (or $XDG_CONFIG_HOME/noob/…)

use std::path::{Path, PathBuf};

use super::model::Settings;

/// Which kind of computer, for working out the settings folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    Mac,
    Windows,
    Linux,
}

impl System {
    pub fn current() -> System {
        if cfg!(target_os = "macos") {
            System::Mac
        } else if cfg!(windows) {
            System::Windows
        } else {
            System::Linux
        }
    }
}

/// The settings file's path. `env` looks up an environment variable (passed in
/// so tests don't depend on the computer they run on). `None` if there's no
/// home folder to put it in.
pub fn settings_path(system: System, env: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let var = |name: &str| {
        env(name)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let folder = match system {
        System::Mac => var("HOME")?
            .join("Library")
            .join("Application Support")
            .join("Noob"),
        System::Windows => var("APPDATA")?.join("Noob"),
        System::Linux => var("XDG_CONFIG_HOME")
            .or_else(|| var("HOME").map(|home| home.join(".config")))?
            .join("noob"),
    };
    Some(folder.join("settings.txt"))
}

/// Your settings, or the normal ones if the file is missing or unreadable.
pub fn load(path: &Path) -> Settings {
    std::fs::read_to_string(path)
        .map(|text| Settings::from_text(&text))
        .unwrap_or_default()
}

/// Saves the settings. Writes a temporary file first and then swaps it in,
/// so a crash halfway never leaves half a file behind.
pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let temporary = path.with_extension("txt.new");
    std::fs::write(&temporary, settings.to_text())?;
    std::fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    #[test]
    fn a_mac_keeps_it_in_application_support() {
        let path = settings_path(System::Mac, env(&[("HOME", "/Users/anand")]));
        assert_eq!(
            path,
            Some(PathBuf::from(
                "/Users/anand/Library/Application Support/Noob/settings.txt"
            ))
        );
    }

    #[test]
    fn windows_keeps_it_in_appdata() {
        let path = settings_path(
            System::Windows,
            env(&[("APPDATA", r"C:\Users\anand\AppData\Roaming")]),
        );
        assert_eq!(
            path,
            Some(
                PathBuf::from(r"C:\Users\anand\AppData\Roaming")
                    .join("Noob")
                    .join("settings.txt")
            )
        );
    }

    #[test]
    fn linux_uses_xdg_config_home_or_dot_config() {
        assert_eq!(
            settings_path(
                System::Linux,
                env(&[("XDG_CONFIG_HOME", "/cfg"), ("HOME", "/home/a")])
            ),
            Some(PathBuf::from("/cfg").join("noob").join("settings.txt"))
        );
        assert_eq!(
            settings_path(System::Linux, env(&[("HOME", "/home/a")])),
            Some(
                PathBuf::from("/home/a")
                    .join(".config")
                    .join("noob")
                    .join("settings.txt")
            )
        );
    }

    #[test]
    fn no_home_folder_means_nowhere_to_save() {
        assert_eq!(settings_path(System::Mac, env(&[])), None);
        assert_eq!(settings_path(System::Linux, env(&[("HOME", "")])), None);
    }

    #[test]
    fn saved_settings_load_back_and_missing_files_load_the_normal_ones() {
        let folder =
            std::env::temp_dir().join(format!("noob-settings-test-{}", std::process::id()));
        let path = folder.join("deeper").join("settings.txt");
        assert_eq!(load(&path), Settings::default(), "no file yet");

        let settings = Settings::default().bigger().bigger();
        save(&path, &settings).expect("saves, making the folders it needs");
        assert_eq!(load(&path), settings);
        let _ = std::fs::remove_dir_all(&folder);
    }
}
