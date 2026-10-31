//! Teaching tests: real bytes that real programs print, and what the parser
//! should turn them into.

use super::*;

/// Feeds everything in one go.
fn parse(bytes: &[u8]) -> Vec<Action> {
    Parser::new().feed(bytes)
}

/// Feeds one byte at a time, the worst way the shell's output could arrive.
fn parse_byte_by_byte(bytes: &[u8]) -> Vec<Action> {
    let mut parser = Parser::new();
    bytes
        .iter()
        .flat_map(|byte| parser.feed(&[*byte]))
        .collect()
}

/// Shorthand for "print these letters".
fn prints(text: &str) -> Vec<Action> {
    text.chars().map(Action::Print).collect()
}

/// Shorthand for a CSI command with plain numbers (no colon parts).
fn csi(private: Option<char>, numbers: &[u16], final_char: char) -> Action {
    Action::Csi(Csi {
        private,
        params: numbers.iter().map(|number| vec![*number]).collect(),
        intermediates: String::new(),
        final_char,
    })
}

fn esc(intermediates: &str, final_char: char) -> Action {
    Action::Esc(Esc {
        intermediates: intermediates.to_string(),
        final_char,
    })
}

/// A tiny, repeatable "random number" maker, so the garbage tests are the same
/// every run (and we don't need a crate for randomness).
struct Dice(u64);

impl Dice {
    fn roll(&mut self) -> u64 {
        // The classic "linear congruential" trick: multiply, add, keep the top bits.
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
}

// ── Plain text and control codes ───────────────────────────────────────────

#[test]
fn plain_text_is_printed_letter_by_letter() {
    assert_eq!(parse(b"hi!"), prints("hi!"));
}

#[test]
fn control_codes_become_control_actions() {
    assert_eq!(
        parse(b"a\x07\x08\t\r\n"),
        vec![
            Action::Print('a'),
            Action::Control(Control::Bell),
            Action::Control(Control::Backspace),
            Action::Control(Control::Tab),
            Action::Control(Control::CarriageReturn),
            Action::Control(Control::LineFeed),
        ]
    );
}

#[test]
fn nothing_bytes_and_delete_are_ignored() {
    // Byte 0 ("NUL") is sometimes sent as padding, and 127 ("DEL") means nothing
    // on output. Real terminals skip both.
    assert_eq!(parse(b"a\x00b\x7fc"), prints("abc"));
}

// ── Real programs ──────────────────────────────────────────────────────────

#[test]
fn a_colour_ls_turns_into_colours_and_names() {
    // `ls --color` on Linux: a folder in bold blue, then a plain file.
    let output = b"\x1b[0m\x1b[01;34mDocuments\x1b[0m  notes.txt\r\n";
    let mut expected = vec![csi(None, &[0], 'm'), csi(None, &[1, 34], 'm')];
    expected.extend(prints("Documents"));
    expected.push(csi(None, &[0], 'm'));
    expected.extend(prints("  notes.txt"));
    expected.push(Action::Control(Control::CarriageReturn));
    expected.push(Action::Control(Control::LineFeed));
    assert_eq!(parse(output), expected);
}

#[test]
fn a_zsh_prompt_sets_the_title_turns_on_paste_mode_and_draws_a_coloured_arrow() {
    // What zsh (with a common theme) prints before you can type:
    //   ESC]2;…BEL   window title          ESC[?2004h  "bracketed paste" on
    //   ESC[01;32m   bold green arrow      ESC[36m     cyan folder name
    //   ESC[00m      back to normal        ESC[K       erase the rest of the line
    let output = "\x1b]2;anand@mac: ~\x07\x1b[?2004h\x1b[01;32m➜  \x1b[36m~\x1b[00m \x1b[K";
    let mut expected = vec![
        Action::Osc(vec!["2".into(), "anand@mac: ~".into()]),
        csi(Some('?'), &[2004], 'h'),
        csi(None, &[1, 32], 'm'),
    ];
    expected.extend(prints("➜  "));
    expected.push(csi(None, &[36], 'm'));
    expected.extend(prints("~"));
    expected.push(csi(None, &[0], 'm'));
    expected.extend(prints(" "));
    expected.push(csi(None, &[], 'K'));
    assert_eq!(parse(output.as_bytes()), expected);
}

#[test]
fn clear_moves_home_and_erases_the_screen_and_the_scrollback() {
    // What `clear` prints on a Mac: erase scrollback, cursor home, erase screen.
    assert_eq!(
        parse(b"\x1b[3J\x1b[H\x1b[2J"),
        vec![
            csi(None, &[3], 'J'),
            csi(None, &[], 'H'),
            csi(None, &[2], 'J')
        ]
    );
}

#[test]
fn cursor_moves_keep_their_numbers() {
    assert_eq!(
        parse(b"\x1b[10;20H\x1b[5A\x1b[;7H"),
        vec![
            // Row 10, column 20.
            csi(None, &[10, 20], 'H'),
            // Up 5 lines.
            csi(None, &[5], 'A'),
            // An empty number counts as 0 ("the usual value", row 1 here).
            csi(None, &[0, 7], 'H'),
        ]
    );
}

#[test]
fn nano_and_htop_switch_to_the_alternate_screen_and_back() {
    // The start of what `nano` prints: alternate screen on, scroll region rows
    // 1-24, normal US letters (`ESC ( B`), reset colours, keypad mode (`ESC =`).
    let start = b"\x1b[?1049h\x1b[1;24r\x1b(B\x1b[m\x1b[?1h\x1b=";
    assert_eq!(
        parse(start),
        vec![
            csi(Some('?'), &[1049], 'h'),
            csi(None, &[1, 24], 'r'),
            esc("(", 'B'),
            csi(None, &[], 'm'),
            csi(Some('?'), &[1], 'h'),
            esc("", '='),
        ]
    );
    // And when you quit: alternate screen off, back to your shell's screen.
    assert_eq!(parse(b"\x1b[?1049l"), vec![csi(Some('?'), &[1049], 'l')]);
}

#[test]
fn short_escape_commands_save_and_restore_the_cursor() {
    assert_eq!(
        parse(b"\x1b7\x1b8\x1bM\x1bc"),
        vec![esc("", '7'), esc("", '8'), esc("", 'M'), esc("", 'c')]
    );
}

#[test]
fn a_cursor_shape_command_keeps_its_in_between_space() {
    // `ESC[2 q` = "steady block cursor": the space is an "intermediate".
    assert_eq!(
        parse(b"\x1b[2 q"),
        vec![Action::Csi(Csi {
            private: None,
            params: vec![vec![2]],
            intermediates: " ".into(),
            final_char: 'q',
        })]
    );
}

#[test]
fn true_colour_with_colons_keeps_the_parts_together() {
    // `ESC[38:2:255:100:0m` = orange text, one number in five parts.
    let actions = parse(b"\x1b[38:2:255:100:0;1m");
    let Action::Csi(command) = &actions[0] else {
        panic!("expected a CSI, got {actions:?}");
    };
    assert_eq!(command.params, vec![vec![38, 2, 255, 100, 0], vec![1]]);
    assert_eq!(command.numbers(), vec![38, 1]);
}

// ── Window titles (OSC) ────────────────────────────────────────────────────

#[test]
fn a_window_title_can_end_with_bell_or_with_esc_backslash() {
    let title = |text: &str| Action::Osc(vec!["0".into(), text.into()]);
    // Ending with BEL (byte 7), the common way.
    assert_eq!(parse(b"\x1b]0;my project\x07"), vec![title("my project")]);
    // Ending with "String Terminator" (ESC \), the official way. The ESC \ is
    // just the end marker, so it doesn't become an action of its own.
    assert_eq!(
        parse(b"\x1b]0;my project\x1b\\ok"),
        [vec![title("my project")], prints("ok")].concat()
    );
}

#[test]
fn a_window_title_can_hold_emoji() {
    assert_eq!(
        parse("\x1b]2;🚀 build\x07".as_bytes()),
        vec![Action::Osc(vec!["2".into(), "🚀 build".into()])]
    );
}

// ── Letters beyond A-Z (UTF-8) ─────────────────────────────────────────────

#[test]
fn emoji_and_other_languages_print_as_whole_letters() {
    // "é" is 2 bytes, "世" is 3, "👋" is 4. Each must print as ONE letter.
    assert_eq!(parse("héllo 世界 👋".as_bytes()), prints("héllo 世界 👋"));
}

#[test]
fn a_letter_cut_in_half_between_two_reads_still_prints_once() {
    let wave = "👋".as_bytes(); // F0 9F 91 8B
    let mut parser = Parser::new();
    assert_eq!(parser.feed(&wave[..2]), vec![], "half a letter: wait");
    assert_eq!(parser.feed(&wave[2..]), prints("👋"));
}

#[test]
fn broken_utf8_shows_the_replacement_mark_instead_of_crashing() {
    // 0xFF is never valid; 0xC3 starts a 2-byte letter but "b" doesn't finish it;
    // ED A0 80 would be a "surrogate", which UTF-8 forbids.
    assert_eq!(parse(b"a\xffb"), prints("a\u{FFFD}b"));
    assert_eq!(parse(b"\xc3b"), prints("\u{FFFD}b"));
    assert_eq!(parse(b"\xed\xa0\x80!"), prints("\u{FFFD}!"));
}

#[test]
fn an_escape_in_the_middle_of_a_letter_still_works() {
    assert_eq!(
        parse(b"\xe4\xb8\x1b[1m"),
        vec![Action::Print('\u{FFFD}'), csi(None, &[1], 'm')]
    );
}

// ── Split anywhere ─────────────────────────────────────────────────────────

/// Everything from the tests above, glued together.
fn everything() -> Vec<u8> {
    [
        &b"\x1b[0m\x1b[01;34mDocuments\x1b[0m  notes.txt\r\n"[..],
        "\x1b]2;anand@mac: ~\x07\x1b[?2004h\x1b[01;32m➜  \x1b[36m~\x1b[00m \x1b[K".as_bytes(),
        b"\x1b[3J\x1b[H\x1b[2J\x1b[10;20H\x1b[5A\x1b[;7H",
        b"\x1b[?1049h\x1b[1;24r\x1b(B\x1b[m\x1b[?1h\x1b=\x1b[?1049l",
        b"\x1b7\x1b8\x1b[2 q\x1b[38:2:255:100:0;1m",
        b"\x1b]0;my project\x1b\\ok\x1bPq#0;2;0;0;0\x1b\\",
        "héllo 世界 👋 \x1b]2;🚀 build\x07".as_bytes(),
        b"a\xffb\xc3b\xed\xa0\x80!\xe4\xb8\x1b[1m",
    ]
    .concat()
}

#[test]
fn feeding_byte_by_byte_gives_the_same_result_as_all_at_once() {
    let bytes = everything();
    assert_eq!(parse_byte_by_byte(&bytes), parse(&bytes));
}

#[test]
fn feeding_in_random_sized_pieces_gives_the_same_result() {
    let bytes = everything();
    let expected = parse(&bytes);
    let mut dice = Dice(7);
    for _ in 0..200 {
        let mut parser = Parser::new();
        let mut actions = Vec::new();
        let mut rest = &bytes[..];
        while !rest.is_empty() {
            let size = (dice.roll() as usize % 9 + 1).min(rest.len());
            actions.extend(parser.feed(&rest[..size]));
            rest = &rest[size..];
        }
        assert_eq!(actions, expected);
    }
}

// ── Bad input never crashes ────────────────────────────────────────────────

#[test]
fn cancel_stops_a_half_finished_command() {
    // Byte 24 ("CAN") abandons the sequence; what follows is plain text again.
    assert_eq!(parse(b"\x1b[31\x18A"), prints("A"));
}

#[test]
fn a_control_code_inside_a_command_still_happens() {
    // Old terminals did this, so programs rely on it: the line feed takes effect
    // and the colour command carries on around it.
    assert_eq!(
        parse(b"\x1b[3\n1m"),
        vec![Action::Control(Control::LineFeed), csi(None, &[31], 'm')]
    );
}

#[test]
fn huge_numbers_stop_at_the_biggest_one_we_keep() {
    assert_eq!(
        parse(b"\x1b[99999999999999999999A"),
        vec![csi(None, &[u16::MAX], 'A')]
    );
}

#[test]
fn a_command_with_far_too_many_numbers_is_dropped() {
    let mut bytes = b"\x1b[".to_vec();
    bytes.extend("1;".repeat(1000).as_bytes());
    bytes.extend(b"mok");
    assert_eq!(parse(&bytes), prints("ok"));
}

#[test]
fn an_endless_title_is_dropped_instead_of_eating_all_memory() {
    // Half a title (or half a link, or half a clipboard paste) would be wrong,
    // so a string that's too long is thrown away whole.
    let mut bytes = b"\x1b]0;".to_vec();
    bytes.extend(std::iter::repeat_n(b'x', 1_000_000));
    bytes.extend(b"\x07ok");
    assert_eq!(parse(&bytes), prints("ok"));
}

#[test]
fn a_letter_inside_an_escape_code_breaks_it_off_and_still_prints() {
    // "é" can't be part of an escape code, so the code is abandoned and the
    // letter shows up as normal text (like xterm). The "m" after it is plain text too.
    assert_eq!(parse("\x1b[1ém".as_bytes()), prints("ém"));
    assert_eq!(parse("\x1béx".as_bytes()), prints("éx"));
}

#[test]
fn image_data_and_other_strings_we_dont_use_are_skipped() {
    // `ESC P … ESC \` ("DCS") carries things like sixel pictures. Skipped for now.
    assert_eq!(parse(b"\x1bPq#0;2;0;0;0#0~~\x1b\\hi"), prints("hi"));
    // `ESC _ … ESC \` ("APC") is skipped too.
    assert_eq!(parse(b"\x1b_secret\x1b\\hi"), prints("hi"));
}

#[test]
fn random_garbage_never_crashes_the_parser() {
    let mut dice = Dice(42);
    let mut parser = Parser::new();
    for _ in 0..2_000 {
        // Lots of ESC, [, ], digits and high bytes, to hit every state.
        let chunk: Vec<u8> = (0..64)
            .map(|_| match dice.roll() % 6 {
                0 => 0x1b,
                1 => b"[];:?P\\"[dice.roll() as usize % 7],
                2 => b'0' + (dice.roll() % 10) as u8,
                _ => dice.roll() as u8,
            })
            .collect();
        parser.feed(&chunk);
    }
    // Afterwards it still understands normal text.
    let actions = parser.feed(b"\x18\x1b\\ok");
    assert!(actions.ends_with(&prints("ok")), "got {actions:?}");
}
