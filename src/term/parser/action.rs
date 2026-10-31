//! What the parser hands back: a list of plain instructions, one per thing it
//! understood in the shell's output.
//!
//! The parser doesn't *do* any of them (it has no screen). Later, Noob's screen
//! grid (stage 3) reads these and moves the cursor, paints colours and so on.

/// One thing the shell asked the terminal to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Show this letter at the cursor (then move the cursor one step right).
    Print(char),
    /// One of the old single-byte "control codes" from teletype days.
    Control(Control),
    /// A "Control Sequence" (`ESC [ … letter`): cursor moves, erasing, colours, modes.
    Csi(Csi),
    /// A short escape command (`ESC` + a letter or two): save the cursor, reset…
    Esc(Esc),
    /// An "Operating System Command" (`ESC ] … BEL`): mostly the window title.
    ///
    /// The text is split at every `;`, so `ESC ]0;my title BEL` becomes
    /// `["0", "my title"]`. The first piece says what the rest means.
    Osc(Vec<String>),
}

/// The control codes a terminal understands. They're the bytes below 32, from
/// the days when terminals were typewriters: ring the bell, go back one space…
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// Byte 7: beep (or flash).
    Bell,
    /// Byte 8: move the cursor one step left.
    Backspace,
    /// Byte 9: jump right to the next tab stop.
    Tab,
    /// Byte 10 (and 11, 12, which terminals treat the same): go down one line.
    LineFeed,
    /// Byte 13: go back to the start of the line.
    CarriageReturn,
    /// Any other control code. Terminals mostly ignore them, so we just pass them on.
    Other(u8),
}

/// A Control Sequence: `ESC [`, maybe a marker like `?`, some numbers split by `;`,
/// then one final letter that says what to do with the numbers.
///
/// Examples you'll see from real programs:
///   - `ESC[10;20H` → move the cursor to row 10, column 20 (`final_char: 'H'`)
///   - `ESC[1;34m`  → bold, blue text (`m` means "set colours and styles")
///   - `ESC[2J`     → erase the whole screen
///   - `ESC[?1049h` → switch to the alternate screen (`private: Some('?')`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csi {
    /// A marker straight after `ESC [`: `?`, `>`, `<` or `=`. `?` is the common one
    /// and means "a DEC private mode" (things like hiding the cursor).
    pub private: Option<char>,
    /// The numbers, in order. An empty number (`ESC[;5H`) counts as 0, which
    /// means "use the usual value".
    ///
    /// Each number is a little list because some colour codes split one number
    /// into parts with colons: `ESC[38:2:255:0:0m` (bright red) arrives as one
    /// number with five parts. A plain `31` is just `[31]`.
    pub params: Vec<Vec<u16>>,
    /// Rare in-between symbols before the final letter, like the space in
    /// `ESC[2 q` ("make the cursor a block").
    pub intermediates: String,
    /// The letter that says what to do.
    pub final_char: char,
}

impl Csi {
    /// Just the main numbers, without colon parts: `ESC[1;34m` → `[1, 34]`.
    pub fn numbers(&self) -> Vec<u16> {
        self.params
            .iter()
            .map(|parts| parts.first().copied().unwrap_or(0))
            .collect()
    }
}

/// A short escape command: `ESC`, maybe an in-between symbol, then a final character.
///
/// Examples: `ESC 7` saves the cursor, `ESC 8` brings it back, `ESC M` moves up
/// one line, `ESC c` resets the terminal, and `ESC ( B` picks the normal
/// (US) letters (`intermediates: "("`, `final_char: 'B'`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Esc {
    pub intermediates: String,
    pub final_char: char,
}
