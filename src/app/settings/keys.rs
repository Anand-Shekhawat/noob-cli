//! Settings shortcuts: Cmd+ / Cmd− / Cmd+0 for the letter size, Cmd+, for the
//! Settings panel (Ctrl instead of Cmd on Windows and Linux).

use eframe::egui::{Key, Modifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsAction {
    Bigger,
    Smaller,
    NormalSize,
    OpenPanel,
}

/// The settings action for a key press, if it is one.
pub fn settings_action(key: Key, mods: Modifiers, on_mac: bool) -> Option<SettingsAction> {
    // Cmd on a Mac (Ctrl there belongs to the shell); Ctrl elsewhere.
    let held = if on_mac {
        (mods.mac_cmd || mods.command) && !mods.ctrl
    } else {
        mods.ctrl
    };
    if !held || mods.alt {
        return None;
    }
    match key {
        // The + key is = without Shift.
        Key::Plus | Key::Equals => Some(SettingsAction::Bigger),
        Key::Minus => Some(SettingsAction::Smaller),
        Key::Num0 => Some(SettingsAction::NormalSize),
        Key::Comma => Some(SettingsAction::OpenPanel),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd() -> Modifiers {
        Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        }
    }

    fn ctrl() -> Modifiers {
        Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        }
    }

    #[test]
    fn cmd_plus_minus_and_zero_change_the_letter_size_on_a_mac() {
        assert_eq!(
            settings_action(Key::Plus, cmd(), true),
            Some(SettingsAction::Bigger)
        );
        assert_eq!(
            settings_action(Key::Equals, cmd(), true),
            Some(SettingsAction::Bigger),
            "the + key without Shift is ="
        );
        assert_eq!(
            settings_action(Key::Minus, cmd(), true),
            Some(SettingsAction::Smaller)
        );
        assert_eq!(
            settings_action(Key::Num0, cmd(), true),
            Some(SettingsAction::NormalSize)
        );
        assert_eq!(
            settings_action(Key::Comma, cmd(), true),
            Some(SettingsAction::OpenPanel)
        );
    }

    #[test]
    fn windows_and_linux_use_ctrl() {
        assert_eq!(
            settings_action(Key::Plus, ctrl(), false),
            Some(SettingsAction::Bigger)
        );
        assert_eq!(
            settings_action(Key::Comma, ctrl(), false),
            Some(SettingsAction::OpenPanel)
        );
    }

    #[test]
    fn shift_held_for_plus_still_counts() {
        let cmd_shift = Modifiers {
            shift: true,
            ..cmd()
        };
        assert_eq!(
            settings_action(Key::Plus, cmd_shift, true),
            Some(SettingsAction::Bigger)
        );
    }

    #[test]
    fn plain_keys_and_ctrl_on_a_mac_go_to_the_shell() {
        assert_eq!(settings_action(Key::Minus, Modifiers::NONE, true), None);
        assert_eq!(
            settings_action(Key::Minus, ctrl(), true),
            None,
            "Ctrl+- on a Mac belongs to the shell"
        );
        assert_eq!(settings_action(Key::A, cmd(), true), None);
    }
}
