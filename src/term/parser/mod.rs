//! Reading what the shell prints.
//!
//! A shell doesn't send a picture of the screen. It sends a stream of bytes: mostly
//! letters to show, mixed with little hidden instructions called "escape codes".
//! Every escape code starts with the byte ESC (27), for example `ESC[1;34m`
//! ("bold blue text from here on") or `ESC[2J` ("erase the screen").
//!
//! The parser reads that stream and turns it into a list of [`Action`]s. It's a
//! "state machine": it's always in one *state* (for example "reading normal
//! text" or "in the middle of an `ESC[` command"), and each byte either does
//! something, moves it to another state, or both. The states and rules follow
//! Paul Williams' well-known chart of how real DEC VT500 terminals did it
//! (vt100.net/emu/dec_ansi_parser), which most modern terminals copy.
//!
//! Two things make it tricky, and both are handled here:
//!   - Output arrives in random-sized pieces, so a command or a letter can be cut
//!     in half. The parser remembers where it was between calls to `feed`.
//!   - Programs sometimes print nonsense. Nothing a program prints may crash
//!     Noob, so bad bytes are skipped or shown as �, and every buffer has a limit.
//!
//! Two simplifications compared with old hardware terminals:
//!   - We always read UTF-8, so bytes 128-159 are parts of letters, never the
//!     old one-byte "C1" commands (modern terminals do the same).
//!   - Strings we have no use for yet (`ESC P` "DCS", used for things like
//!     pictures, and `ESC X`, `ESC ^`, `ESC _`) are skipped whole. Williams' chart
//!     has several DCS states; for skipping, one state is enough.

mod action;
mod utf8;

pub use action::{Action, Control, Csi, Esc};
use utf8::{REPLACEMENT, Utf8, is_continuation};

/// The most numbers we keep in one CSI command. Real ones use a handful (a long
/// colour command maybe 10); more than this is nonsense, and the command is dropped.
const MAX_PARAMS: usize = 32;

/// The most colon-parts in one number (`38:2:0:255:100:0` has 6).
const MAX_PARTS: usize = 8;

/// The most in-between symbols in one command (real ones use 0 or 1).
const MAX_INTERMEDIATES: usize = 2;

/// The most bytes we keep of one OSC string (a window title). Anything longer
/// is cut off, so a program can't make Noob use up all the memory.
const MAX_OSC_BYTES: usize = 64 * 1024;

/// The byte that starts every escape code.
const ESC: u8 = 0x1b;
/// "Cancel" and "Substitute": both abandon a half-finished escape code.
const CAN: u8 = 0x18;
const SUB: u8 = 0x1a;
const BEL: u8 = 0x07;

/// Where the parser is in the stream (see Williams' chart for the full picture).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum State {
    /// Reading normal text.
    #[default]
    Ground,
    /// Just saw ESC; the next byte says what kind of escape code this is.
    Escape,
    /// `ESC` plus an in-between symbol, like the `(` in `ESC ( B`.
    EscapeIntermediate,
    /// Just saw `ESC [`.
    CsiEntry,
    /// Reading the numbers of a CSI command.
    CsiParam,
    /// Reading in-between symbols, after the numbers.
    CsiIntermediate,
    /// The command is broken; skip it until its final letter.
    CsiIgnore,
    /// Inside `ESC ] … BEL`, collecting the text.
    OscString,
    /// Inside a string we don't use (DCS and friends): skip until it ends.
    SkipString,
}

/// Turns the shell's bytes into [`Action`]s. Keep one per terminal and feed it
/// every piece of output, in order.
#[derive(Debug, Default)]
pub struct Parser {
    state: State,
    /// A letter cut in half between two pieces of output.
    utf8: Utf8,
    /// The pieces of the escape code being read right now.
    private: Option<char>,
    params: Vec<Vec<u16>>,
    /// The colon-parts of the number being read, and its digits so far.
    parts: Vec<u16>,
    number: u16,
    /// Whether this command has any numbers at all (`ESC[m` has none).
    has_params: bool,
    intermediates: String,
    /// Too many in-between symbols: drop this command when it ends.
    too_long: bool,
    osc: Vec<u8>,
}

impl Parser {
    pub fn new() -> Parser {
        Parser::default()
    }

    /// Reads the next piece of the shell's output and says what it asks for.
    /// A command cut off at the end waits, and finishes in the next `feed`.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Action> {
        let mut actions = Vec::new();
        for &byte in bytes {
            self.step(byte, &mut actions);
        }
        actions
    }

    /// Handles one byte.
    fn step(&mut self, byte: u8, actions: &mut Vec<Action>) {
        // A letter was started but this byte doesn't continue it: it's broken.
        if self.utf8.is_waiting() && !is_continuation(byte) {
            self.utf8.reset();
            actions.push(Action::Print(REPLACEMENT));
        }

        // Rules that apply in every state ("anywhere" in Williams' chart).
        match byte {
            CAN | SUB => {
                self.state = State::Ground;
                return;
            }
            ESC => {
                // ESC also ends a title (`ESC \` is the official way to end one).
                if self.state == State::OscString {
                    self.finish_osc(actions);
                }
                self.start_escape();
                return;
            }
            _ => {}
        }

        match self.state {
            State::Ground => self.ground(byte, actions),
            State::Escape => self.escape(byte, actions),
            State::EscapeIntermediate => self.escape_intermediate(byte, actions),
            State::CsiEntry => self.csi_entry(byte, actions),
            State::CsiParam => self.csi_param(byte, actions),
            State::CsiIntermediate => self.csi_intermediate(byte, actions),
            State::CsiIgnore => self.csi_ignore(byte, actions),
            State::OscString => self.osc_string(byte, actions),
            // Only ESC (handled above), CAN or SUB end it.
            State::SkipString => {}
        }
    }

    fn ground(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x00..=0x1f => execute(byte, actions),
            0x20..=0x7e => actions.push(Action::Print(byte as char)),
            // DEL means nothing on output.
            0x7f => {}
            // Part of a longer letter: print it once it's complete.
            0x80..=0xff => {
                if let Some(letter) = self.utf8.push(byte) {
                    actions.push(Action::Print(letter));
                }
            }
        }
    }

    fn escape(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x00..=0x1f => execute(byte, actions),
            0x20..=0x2f => {
                self.collect(byte);
                self.state = State::EscapeIntermediate;
            }
            b'[' => self.state = State::CsiEntry,
            b']' => self.state = State::OscString,
            b'P' | b'X' | b'^' | b'_' => self.state = State::SkipString,
            // `ESC \` on its own marks the end of a string: nothing to do.
            b'\\' => self.state = State::Ground,
            0x30..=0x7e => self.dispatch_esc(byte, actions),
            _ => {}
        }
    }

    fn escape_intermediate(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x00..=0x1f => execute(byte, actions),
            0x20..=0x2f => self.collect(byte),
            0x30..=0x7e => self.dispatch_esc(byte, actions),
            _ => {}
        }
    }

    fn csi_entry(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x00..=0x1f => execute(byte, actions),
            0x20..=0x2f => {
                self.state = State::CsiIntermediate;
                self.collect(byte);
            }
            // Digits, `:` and `;`.
            0x30..=0x3b => {
                self.state = State::CsiParam;
                self.param(byte);
            }
            // A private marker (`?` and friends) only counts straight after `ESC [`.
            0x3c..=0x3f => {
                self.private = Some(byte as char);
                self.state = State::CsiParam;
            }
            0x40..=0x7e => self.dispatch_csi(byte, actions),
            _ => {}
        }
    }

    fn csi_param(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x00..=0x1f => execute(byte, actions),
            0x30..=0x3b => self.param(byte),
            // A marker in the middle of the numbers makes no sense.
            0x3c..=0x3f => self.state = State::CsiIgnore,
            0x20..=0x2f => {
                self.state = State::CsiIntermediate;
                self.collect(byte);
            }
            0x40..=0x7e => self.dispatch_csi(byte, actions),
            _ => {}
        }
    }

    fn csi_intermediate(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x00..=0x1f => execute(byte, actions),
            0x20..=0x2f => self.collect(byte),
            // Numbers after the in-between symbols: broken.
            0x30..=0x3f => self.state = State::CsiIgnore,
            0x40..=0x7e => self.dispatch_csi(byte, actions),
            _ => {}
        }
    }

    fn csi_ignore(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            0x00..=0x1f => execute(byte, actions),
            // The final letter ends the broken command; we do nothing with it.
            0x40..=0x7e => self.state = State::Ground,
            _ => {}
        }
    }

    fn osc_string(&mut self, byte: u8, actions: &mut Vec<Action>) {
        match byte {
            // BEL ends a title (the way xterm started, and everyone copied).
            BEL => {
                self.finish_osc(actions);
                self.state = State::Ground;
            }
            // Other control codes inside a title are ignored.
            0x00..=0x1f => {}
            _ => {
                if self.osc.len() < MAX_OSC_BYTES {
                    self.osc.push(byte);
                }
            }
        }
    }

    /// Forgets the previous escape code and starts a new one.
    fn start_escape(&mut self) {
        self.state = State::Escape;
        self.private = None;
        self.params.clear();
        self.parts.clear();
        self.number = 0;
        self.has_params = false;
        self.intermediates.clear();
        self.too_long = false;
        self.osc.clear();
    }

    /// Keeps an in-between symbol (or notes that there are too many).
    fn collect(&mut self, byte: u8) {
        if self.intermediates.len() < MAX_INTERMEDIATES {
            self.intermediates.push(byte as char);
        } else {
            self.too_long = true;
        }
    }

    /// Reads one byte of the numbers: a digit, `:` (next part) or `;` (next number).
    fn param(&mut self, byte: u8) {
        self.has_params = true;
        match byte {
            b'0'..=b'9' => {
                // Numbers that grow too big stop at the biggest one we keep.
                let digit = u16::from(byte - b'0');
                self.number = self.number.saturating_mul(10).saturating_add(digit);
            }
            b':' => {
                if self.parts.len() + 1 < MAX_PARTS {
                    self.parts.push(self.number);
                    self.number = 0;
                } else {
                    self.state = State::CsiIgnore;
                }
            }
            // `;`
            _ => {
                if !self.finish_param() {
                    self.state = State::CsiIgnore;
                }
            }
        }
    }

    /// Stores the number just read. Says `false` if there are too many numbers.
    fn finish_param(&mut self) -> bool {
        if self.params.len() >= MAX_PARAMS {
            return false;
        }
        self.parts.push(self.number);
        self.params.push(std::mem::take(&mut self.parts));
        self.number = 0;
        true
    }

    /// The final letter of a CSI command arrived: hand over the whole command.
    fn dispatch_csi(&mut self, byte: u8, actions: &mut Vec<Action>) {
        self.state = State::Ground;
        if self.too_long || (self.has_params && !self.finish_param()) {
            return;
        }
        actions.push(Action::Csi(Csi {
            private: self.private,
            params: std::mem::take(&mut self.params),
            intermediates: std::mem::take(&mut self.intermediates),
            final_char: byte as char,
        }));
    }

    /// The final character of a short escape command arrived.
    fn dispatch_esc(&mut self, byte: u8, actions: &mut Vec<Action>) {
        self.state = State::Ground;
        if self.too_long {
            return;
        }
        actions.push(Action::Esc(Esc {
            intermediates: std::mem::take(&mut self.intermediates),
            final_char: byte as char,
        }));
    }

    /// A title (or other OSC string) ended: split it at `;` and hand it over.
    fn finish_osc(&mut self, actions: &mut Vec<Action>) {
        let parts = self
            .osc
            .split(|byte| *byte == b';')
            .map(|part| String::from_utf8_lossy(part).into_owned())
            .collect();
        self.osc.clear();
        actions.push(Action::Osc(parts));
    }
}

/// Carries out a control code (byte below 32). Byte 0 is padding and does nothing.
fn execute(byte: u8, actions: &mut Vec<Action>) {
    let control = match byte {
        0x00 => return,
        BEL => Control::Bell,
        0x08 => Control::Backspace,
        0x09 => Control::Tab,
        // Line feed, plus "vertical tab" and "form feed", which act the same.
        0x0a..=0x0c => Control::LineFeed,
        0x0d => Control::CarriageReturn,
        other => Control::Other(other),
    };
    actions.push(Action::Control(control));
}

#[cfg(test)]
mod tests;
