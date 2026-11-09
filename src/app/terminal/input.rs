//! Deciding what each keyboard event means for the terminal.
//!
//! egui hands us a list of events each frame: text that was typed, keys, and
//! "copy" / "paste" (which it makes from Cmd+C / Cmd+V on a Mac, and from
//! Ctrl+C / Ctrl+V on Windows and Linux). Here we turn them into requests.
//! It's a pure function, so the tricky cases are easy to test.
//!
//! The tricky case: on Windows and Linux, Ctrl+C is both "copy" and "stop the
//! program". Like Windows Terminal, we copy when something is selected and
//! otherwise send Ctrl+C to the shell.

use eframe::egui::{Event, Key, Modifiers};

use super::keys::{key_bytes, paste_bytes};

/// Ctrl+C and Ctrl+X as bytes, for when egui turned them into copy / cut.
const CTRL_C: u8 = 0x03;
const CTRL_X: u8 = 0x18;

/// What the terminal should do about an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Type these bytes into the shell.
    Send(Vec<u8>),
    /// Put the selected text on the clipboard.
    Copy,
    /// Ask the system for the clipboard; it arrives later as a paste event.
    AskForPaste,
}

/// What a frame's keyboard events ask for, in order.
///
/// `on_mac` matters because egui uses Cmd for copy and paste there, so Ctrl+C
/// always reaches us as a key, never as "copy".
pub fn requests(
    events: &[Event],
    app_cursor: bool,
    bracketed_paste: bool,
    has_selection: bool,
    on_mac: bool,
) -> Vec<Request> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::Text(text) => Some(Request::Send(text.as_bytes().to_vec())),
            Event::Paste(text) => Some(Request::Send(paste_bytes(text, bracketed_paste))),
            Event::Copy if has_selection => Some(Request::Copy),
            Event::Copy if !on_mac => Some(Request::Send(vec![CTRL_C])),
            Event::Cut if !on_mac => Some(Request::Send(vec![CTRL_X])),
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => key_request(*key, *modifiers, app_cursor),
            _ => None,
        })
        .collect()
}

fn key_request(key: Key, mods: Modifiers, app_cursor: bool) -> Option<Request> {
    // Linux terminals copy and paste with Ctrl+Shift+C / V.
    let ctrl_shift = mods.ctrl && mods.shift && !mods.alt;
    match key {
        Key::C if ctrl_shift => Some(Request::Copy),
        Key::V if ctrl_shift => Some(Request::AskForPaste),
        _ => key_bytes(key, mods, app_cursor).map(Request::Send),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn ctrl() -> Modifiers {
        Modifiers {
            ctrl: true,
            ..Modifiers::NONE
        }
    }

    fn send(bytes: &[u8]) -> Request {
        Request::Send(bytes.to_vec())
    }

    #[test]
    fn typed_text_and_keys_become_bytes_in_order() {
        let events = [
            Event::Text("ls".into()),
            key(Key::L, Modifiers::NONE), // the key behind the text: ignored
            key(Key::Enter, Modifiers::NONE),
        ];
        assert_eq!(
            requests(&events, false, false, false, true),
            vec![send(b"ls"), send(b"\r")]
        );
    }

    #[test]
    fn released_keys_send_nothing() {
        let released = Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        assert!(requests(&[released], false, false, false, true).is_empty());
    }

    #[test]
    fn ctrl_c_on_a_mac_always_stops_the_program() {
        let events = [key(Key::C, ctrl())];
        assert_eq!(
            requests(&events, false, false, true, true),
            vec![send(&[3])]
        );
    }

    #[test]
    fn copy_with_a_selection_copies_it() {
        assert_eq!(
            requests(&[Event::Copy], false, false, true, false),
            vec![Request::Copy]
        );
    }

    #[test]
    fn ctrl_c_on_windows_without_a_selection_stops_the_program() {
        assert_eq!(
            requests(&[Event::Copy], false, false, false, false),
            vec![send(&[3])]
        );
        assert_eq!(
            requests(&[Event::Cut], false, false, false, false),
            vec![send(&[0x18])]
        );
    }

    #[test]
    fn cmd_c_on_a_mac_without_a_selection_does_nothing() {
        assert!(requests(&[Event::Copy], false, false, false, true).is_empty());
    }

    #[test]
    fn paste_respects_bracketed_paste() {
        let events = [Event::Paste("pwd\n".into())];
        assert_eq!(
            requests(&events, false, true, false, true),
            vec![send(b"\x1b[200~pwd\r\x1b[201~")]
        );
        assert_eq!(
            requests(&events, false, false, false, true),
            vec![send(b"pwd\r")]
        );
    }

    #[test]
    fn ctrl_shift_c_and_v_copy_and_paste_like_linux_terminals() {
        let both = Modifiers {
            ctrl: true,
            shift: true,
            ..Modifiers::NONE
        };
        assert_eq!(
            requests(&[key(Key::C, both)], false, false, true, false),
            vec![Request::Copy]
        );
        assert_eq!(
            requests(&[key(Key::V, both)], false, false, false, false),
            vec![Request::AskForPaste]
        );
    }

    #[test]
    fn arrows_follow_application_cursor_mode() {
        let events = [key(Key::ArrowUp, Modifiers::NONE)];
        assert_eq!(
            requests(&events, true, false, false, true),
            vec![send(b"\x1bOA")]
        );
    }
}
