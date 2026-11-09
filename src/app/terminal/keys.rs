//! Turning key presses into the bytes a shell expects.
//!
//! A terminal doesn't send "the left arrow was pressed". It sends a few bytes
//! that programs agreed on decades ago: Enter is `\r`, Ctrl+C is byte 3, the
//! left arrow is `ESC [ D`. These functions only do that translation: no
//! window, no shell, so they're easy to test.
//!
//! Typed letters arrive separately (as text), so here we only handle keys that
//! don't type a letter by themselves: Enter, arrows, Ctrl+letter and so on.

use eframe::egui::{Key, Modifiers};

const ESC: u8 = 0x1b;

/// Markers around pasted text when the shell asked for "bracketed paste".
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

/// The bytes for one key press, or `None` if the key doesn't go to the shell
/// (plain letters arrive as text instead, and Cmd shortcuts belong to the app).
///
/// `app_cursor` is the "application cursor keys" mode some programs (vim,
/// less) switch on: arrows then send `ESC O A` instead of `ESC [ A`.
pub fn key_bytes(key: Key, mods: Modifiers, app_cursor: bool) -> Option<Vec<u8>> {
    // On a Mac, Cmd+anything is for the app (copy, paste, quit), never the shell.
    if mods.mac_cmd {
        return None;
    }
    // Windows reports AltGr (how many keyboards type @, € or {) as Ctrl+Alt.
    // Those keys mean a letter, not a control code, so send nothing for them.
    if mods.ctrl && mods.alt && ctrl_byte(key).is_some() {
        return None;
    }
    if mods.ctrl
        && !mods.shift
        && let Some(byte) = ctrl_byte(key)
    {
        return Some(with_alt(mods, vec![byte]));
    }
    if let Some(bytes) = plain_key(key, mods) {
        return Some(with_alt(mods, bytes));
    }
    if let Some(letter) = cursor_letter(key) {
        return Some(cursor_key(letter, mods, app_cursor));
    }
    if let Some(number) = tilde_number(key) {
        return Some(tilde_key(number, mods));
    }
    function_key(key, mods)
}

/// What to send when text is pasted. With bracketed paste on, the text is
/// wrapped in markers so the shell knows not to run each line straight away.
/// New lines become `\r`, which is what pressing Enter sends.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    let text = text.replace("\r\n", "\r").replace('\n', "\r");
    if !bracketed {
        return text.into_bytes();
    }
    // Pasted text must not be able to end the paste early and type commands.
    // Removing just the end marker isn't enough (`ESC[20` + marker + `1~`
    // rebuilds one), so every escape character goes, including the one-byte
    // form some terminals accept.
    let text: String = text
        .chars()
        .filter(|letter| *letter != '\x1b' && *letter != '\u{9b}')
        .collect();
    [PASTE_START, text.as_bytes(), PASTE_END].concat()
}

/// Ctrl+letter is the letter's place in the alphabet: Ctrl+A is 1, Ctrl+C
/// is 3 (stop), Ctrl+D is 4 (end of input). A few symbols have codes too.
fn ctrl_byte(key: Key) -> Option<u8> {
    if let [letter] = key.name().as_bytes()
        && letter.is_ascii_uppercase()
    {
        return Some(letter - b'A' + 1);
    }
    match key {
        Key::Space | Key::Num2 => Some(0),
        Key::OpenBracket | Key::Num3 => Some(ESC),
        Key::Backslash | Key::Num4 => Some(0x1c),
        Key::CloseBracket | Key::Num5 => Some(0x1d),
        Key::Num6 => Some(0x1e),
        Key::Minus | Key::Slash | Key::Num7 => Some(0x1f),
        Key::Backspace | Key::Num8 => Some(0x08),
        _ => None,
    }
}

/// Keys with a fixed code of their own.
fn plain_key(key: Key, mods: Modifiers) -> Option<Vec<u8>> {
    let bytes: &[u8] = match key {
        Key::Enter => b"\r",
        // Shift+Tab goes backwards (zsh menus, fish).
        Key::Tab if mods.shift => b"\x1b[Z",
        Key::Tab => b"\t",
        // Today's terminals send "delete" (127) for Backspace; shells expect it.
        Key::Backspace => b"\x7f",
        Key::Escape => b"\x1b",
        _ => return None,
    };
    Some(bytes.to_vec())
}

/// Alt (Option on a Mac) in front of a key is sent as ESC first.
fn with_alt(mods: Modifiers, bytes: Vec<u8>) -> Vec<u8> {
    if mods.alt {
        [&[ESC][..], &bytes].concat()
    } else {
        bytes
    }
}

/// The arrows, Home and End end in a letter.
fn cursor_letter(key: Key) -> Option<u8> {
    match key {
        Key::ArrowUp => Some(b'A'),
        Key::ArrowDown => Some(b'B'),
        Key::ArrowRight => Some(b'C'),
        Key::ArrowLeft => Some(b'D'),
        Key::End => Some(b'F'),
        Key::Home => Some(b'H'),
        _ => None,
    }
}

/// Insert, Delete, PgUp and PgDn are `ESC [ number ~`.
fn tilde_number(key: Key) -> Option<u8> {
    match key {
        Key::Insert => Some(2),
        Key::Delete => Some(3),
        Key::PageUp => Some(5),
        Key::PageDown => Some(6),
        _ => None,
    }
}

/// xterm's way of saying which of Shift, Alt and Ctrl are held: 1, plus 1 for
/// Shift, 2 for Alt and 4 for Ctrl. `None` when none are held.
fn modifier_number(mods: Modifiers) -> Option<u8> {
    let number = 1 + u8::from(mods.shift) + 2 * u8::from(mods.alt) + 4 * u8::from(mods.ctrl);
    (number > 1).then_some(number)
}

fn cursor_key(letter: u8, mods: Modifiers, app_cursor: bool) -> Vec<u8> {
    match modifier_number(mods) {
        // Ctrl+Left and friends: `ESC [ 1 ; 5 D`.
        Some(held) => format!("\x1b[1;{held}{}", letter as char).into_bytes(),
        None if app_cursor => vec![ESC, b'O', letter],
        None => vec![ESC, b'[', letter],
    }
}

fn tilde_key(number: u8, mods: Modifiers) -> Vec<u8> {
    match modifier_number(mods) {
        Some(held) => format!("\x1b[{number};{held}~").into_bytes(),
        None => format!("\x1b[{number}~").into_bytes(),
    }
}

/// F1 to F12. F1-F4 are `ESC O P` to `ESC O S`; the rest have numbers.
fn function_key(key: Key, mods: Modifiers) -> Option<Vec<u8>> {
    const KEYS: [Key; 12] = [
        Key::F1,
        Key::F2,
        Key::F3,
        Key::F4,
        Key::F5,
        Key::F6,
        Key::F7,
        Key::F8,
        Key::F9,
        Key::F10,
        Key::F11,
        Key::F12,
    ];
    const NUMBERS: [u8; 8] = [15, 17, 18, 19, 20, 21, 23, 24];
    let index = KEYS.iter().position(|candidate| *candidate == key)?;
    let Some(number) = index.checked_sub(4).map(|later| NUMBERS[later]) else {
        let letter = b"PQRS"[index];
        return Some(match modifier_number(mods) {
            Some(held) => format!("\x1b[1;{held}{}", letter as char).into_bytes(),
            None => vec![ESC, b'O', letter],
        });
    };
    Some(tilde_key(number, mods))
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Modifiers = Modifiers::NONE;

    fn ctrl() -> Modifiers {
        Modifiers {
            ctrl: true,
            ..Modifiers::NONE
        }
    }

    fn send(key: Key, mods: Modifiers) -> Option<Vec<u8>> {
        key_bytes(key, mods, false)
    }

    #[test]
    fn enter_backspace_tab_and_escape_send_their_classic_codes() {
        assert_eq!(send(Key::Enter, NONE), Some(b"\r".to_vec()));
        assert_eq!(send(Key::Backspace, NONE), Some(vec![0x7f]));
        assert_eq!(send(Key::Tab, NONE), Some(b"\t".to_vec()));
        assert_eq!(send(Key::Escape, NONE), Some(vec![0x1b]));
        assert_eq!(send(Key::Tab, Modifiers::SHIFT), Some(b"\x1b[Z".to_vec()));
    }

    #[test]
    fn ctrl_letters_are_their_place_in_the_alphabet() {
        assert_eq!(
            send(Key::C, ctrl()),
            Some(vec![3]),
            "Ctrl+C stops a program"
        );
        assert_eq!(send(Key::D, ctrl()), Some(vec![4]), "Ctrl+D ends input");
        assert_eq!(send(Key::A, ctrl()), Some(vec![1]));
        assert_eq!(send(Key::Z, ctrl()), Some(vec![26]));
        assert_eq!(send(Key::Space, ctrl()), Some(vec![0]));
        assert_eq!(send(Key::OpenBracket, ctrl()), Some(vec![0x1b]));
    }

    #[test]
    fn plain_letters_are_left_to_the_typed_text() {
        assert_eq!(send(Key::A, NONE), None);
        assert_eq!(send(Key::Space, NONE), None);
        assert_eq!(send(Key::A, Modifiers::SHIFT), None);
    }

    #[test]
    fn mac_cmd_shortcuts_never_reach_the_shell() {
        let cmd = Modifiers {
            mac_cmd: true,
            command: true,
            ..Modifiers::NONE
        };
        assert_eq!(send(Key::C, cmd), None);
        assert_eq!(send(Key::ArrowLeft, cmd), None);
    }

    #[test]
    fn arrows_home_and_end_follow_the_cursor_key_mode() {
        assert_eq!(send(Key::ArrowUp, NONE), Some(b"\x1b[A".to_vec()));
        assert_eq!(send(Key::ArrowLeft, NONE), Some(b"\x1b[D".to_vec()));
        assert_eq!(send(Key::Home, NONE), Some(b"\x1b[H".to_vec()));
        assert_eq!(send(Key::End, NONE), Some(b"\x1b[F".to_vec()));
        assert_eq!(
            key_bytes(Key::ArrowUp, NONE, true),
            Some(b"\x1bOA".to_vec()),
            "vim and less switch on application cursor keys"
        );
    }

    #[test]
    fn held_modifiers_are_numbered_like_xterm_does() {
        assert_eq!(send(Key::ArrowRight, ctrl()), Some(b"\x1b[1;5C".to_vec()));
        assert_eq!(
            send(Key::ArrowLeft, Modifiers::SHIFT),
            Some(b"\x1b[1;2D".to_vec())
        );
        assert_eq!(send(Key::Delete, ctrl()), Some(b"\x1b[3;5~".to_vec()));
    }

    #[test]
    fn delete_insert_and_page_keys_end_in_a_tilde() {
        assert_eq!(send(Key::Delete, NONE), Some(b"\x1b[3~".to_vec()));
        assert_eq!(send(Key::Insert, NONE), Some(b"\x1b[2~".to_vec()));
        assert_eq!(send(Key::PageUp, NONE), Some(b"\x1b[5~".to_vec()));
        assert_eq!(send(Key::PageDown, NONE), Some(b"\x1b[6~".to_vec()));
    }

    #[test]
    fn function_keys_use_their_traditional_codes() {
        assert_eq!(send(Key::F1, NONE), Some(b"\x1bOP".to_vec()));
        assert_eq!(send(Key::F4, NONE), Some(b"\x1bOS".to_vec()));
        assert_eq!(send(Key::F5, NONE), Some(b"\x1b[15~".to_vec()));
        assert_eq!(send(Key::F12, NONE), Some(b"\x1b[24~".to_vec()));
    }

    #[test]
    fn alt_puts_escape_in_front() {
        assert_eq!(
            send(Key::Backspace, Modifiers::ALT),
            Some(b"\x1b\x7f".to_vec())
        );
    }

    #[test]
    fn a_hidden_end_marker_cannot_rebuild_itself_inside_a_paste() {
        let sneaky = "a\x1b[20\x1b[201~1~rm -rf x\n";
        let sent = paste_bytes(sneaky, true);
        let inner = &sent[PASTE_START.len()..sent.len() - PASTE_END.len()];
        assert!(
            !inner.contains(&0x1b),
            "no escape codes survive inside a bracketed paste: {inner:?}"
        );
    }

    #[test]
    fn ctrl_alt_letters_send_nothing_because_windows_uses_them_for_altgr() {
        let altgr = Modifiers {
            ctrl: true,
            alt: true,
            ..Modifiers::NONE
        };
        assert_eq!(
            send(Key::Q, altgr),
            None,
            "AltGr+Q types @ on a German keyboard"
        );
        assert_eq!(
            send(Key::ArrowLeft, altgr),
            Some(b"\x1b[1;7D".to_vec()),
            "special keys still work"
        );
    }

    #[test]
    fn pasting_turns_new_lines_into_enter() {
        assert_eq!(paste_bytes("ls\npwd\r\n", false), b"ls\rpwd\r".to_vec());
    }

    #[test]
    fn bracketed_paste_wraps_the_text_and_cannot_be_escaped() {
        assert_eq!(
            paste_bytes("echo hi", true),
            b"\x1b[200~echo hi\x1b[201~".to_vec()
        );
        assert_eq!(
            paste_bytes("a\x1b[201~rm -rf x\n", true),
            b"\x1b[200~a[201~rm -rf x\r\x1b[201~".to_vec(),
            "an end marker inside the paste loses its escape, so it is just text"
        );
    }
}
