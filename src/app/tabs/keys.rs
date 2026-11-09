//! Tab shortcuts: which key presses open, close and switch tabs.
//!
//! On a Mac they use Cmd, which the shell never sees anyway. On Windows and
//! Linux they use Ctrl+Shift, because plain Ctrl+T and Ctrl+W already mean
//! something to the shell (Ctrl+W deletes a word).

use eframe::egui::{Key, Modifiers};

/// What a tab shortcut asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabAction {
    New,
    Close,
    Next,
    Previous,
    /// Jump to a tab: 0 is the first (Cmd+1).
    Select(usize),
    /// Cmd+9 always means the last tab, however many there are.
    Last,
}

/// The key to match shortcuts against: where it is on the keyboard when we
/// know, otherwise the letter it typed.
pub fn shortcut_key(key: Key, physical: Option<Key>) -> Key {
    physical.unwrap_or(key)
}

/// The tab action for a key press, if it is one.
pub fn tab_action(key: Key, mods: Modifiers, on_mac: bool) -> Option<TabAction> {
    // Ctrl+Tab and Ctrl+Shift+Tab, like in web browsers, everywhere.
    if key == Key::Tab && mods.ctrl && !mods.alt && !mods.mac_cmd {
        return Some(if mods.shift {
            TabAction::Previous
        } else {
            TabAction::Next
        });
    }
    let held = if on_mac {
        // `command` is Cmd on a Mac; some sources set only it, not `mac_cmd`.
        (mods.mac_cmd || mods.command) && !mods.ctrl && !mods.alt
    } else {
        mods.ctrl && mods.shift && !mods.alt
    };
    if !held {
        return None;
    }
    const NUMBERS: [Key; 8] = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
    ];
    match key {
        Key::T => Some(TabAction::New),
        Key::W => Some(TabAction::Close),
        Key::CloseBracket | Key::CloseCurlyBracket => Some(TabAction::Next),
        Key::OpenBracket | Key::OpenCurlyBracket => Some(TabAction::Previous),
        Key::Num9 => Some(TabAction::Last),
        _ => NUMBERS
            .iter()
            .position(|number| *number == key)
            .map(TabAction::Select),
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

    fn ctrl_shift() -> Modifiers {
        Modifiers {
            ctrl: true,
            command: true,
            shift: true,
            ..Modifiers::NONE
        }
    }

    #[test]
    fn mac_shortcuts_use_cmd() {
        assert_eq!(tab_action(Key::T, cmd(), true), Some(TabAction::New));
        assert_eq!(tab_action(Key::W, cmd(), true), Some(TabAction::Close));
        assert_eq!(
            tab_action(Key::Num1, cmd(), true),
            Some(TabAction::Select(0))
        );
        assert_eq!(
            tab_action(Key::Num3, cmd(), true),
            Some(TabAction::Select(2))
        );
        assert_eq!(tab_action(Key::Num9, cmd(), true), Some(TabAction::Last));
    }

    #[test]
    fn cmd_counts_whichever_way_it_is_reported() {
        assert_eq!(
            tab_action(Key::T, Modifiers::COMMAND, true),
            Some(TabAction::New)
        );
    }

    #[test]
    fn mac_cmd_shift_brackets_move_between_tabs() {
        let cmd_shift = Modifiers {
            shift: true,
            ..cmd()
        };
        assert_eq!(
            tab_action(Key::CloseBracket, cmd_shift, true),
            Some(TabAction::Next)
        );
        assert_eq!(
            tab_action(Key::OpenBracket, cmd_shift, true),
            Some(TabAction::Previous)
        );
    }

    #[test]
    fn windows_and_linux_use_ctrl_shift_so_the_shell_keeps_ctrl_w() {
        assert_eq!(
            tab_action(Key::T, ctrl_shift(), false),
            Some(TabAction::New)
        );
        assert_eq!(
            tab_action(Key::W, ctrl_shift(), false),
            Some(TabAction::Close)
        );
        assert_eq!(
            tab_action(Key::Num2, ctrl_shift(), false),
            Some(TabAction::Select(1))
        );
        let ctrl = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        assert_eq!(
            tab_action(Key::W, ctrl, false),
            None,
            "Ctrl+W deletes a word"
        );
        assert_eq!(tab_action(Key::T, ctrl, false), None);
    }

    #[test]
    fn ctrl_tab_and_ctrl_shift_tab_switch_tabs_everywhere() {
        let ctrl = Modifiers {
            ctrl: true,
            ..Modifiers::NONE
        };
        let ctrl_shift = Modifiers {
            shift: true,
            ..ctrl
        };
        assert_eq!(tab_action(Key::Tab, ctrl, true), Some(TabAction::Next));
        assert_eq!(
            tab_action(Key::Tab, ctrl_shift, true),
            Some(TabAction::Previous)
        );
        assert_eq!(tab_action(Key::Tab, ctrl, false), Some(TabAction::Next));
    }

    #[test]
    fn shifted_keys_are_read_by_where_they_are_on_the_keyboard() {
        // With Shift held, "]" arrives as "}" and "1" as "!". The key's place
        // on the keyboard (its "physical" key) is still "]" or "1".
        assert_eq!(
            shortcut_key(Key::CloseCurlyBracket, Some(Key::CloseBracket)),
            Key::CloseBracket
        );
        assert_eq!(
            shortcut_key(Key::Exclamationmark, Some(Key::Num1)),
            Key::Num1
        );
        assert_eq!(shortcut_key(Key::T, None), Key::T);
    }

    #[test]
    fn curly_brackets_still_switch_tabs_if_that_is_all_we_get() {
        let cmd_shift = Modifiers {
            shift: true,
            ..cmd()
        };
        assert_eq!(
            tab_action(Key::CloseCurlyBracket, cmd_shift, true),
            Some(TabAction::Next)
        );
        assert_eq!(
            tab_action(Key::OpenCurlyBracket, cmd_shift, true),
            Some(TabAction::Previous)
        );
    }

    #[test]
    fn ordinary_keys_are_not_tab_shortcuts() {
        assert_eq!(tab_action(Key::T, Modifiers::NONE, true), None);
        assert_eq!(tab_action(Key::Num1, Modifiers::NONE, false), None);
        assert_eq!(tab_action(Key::Tab, Modifiers::NONE, true), None);
        assert_eq!(tab_action(Key::Tab, Modifiers::SHIFT, true), None);
    }
}
