//! Putting letters back together from bytes (UTF-8).
//!
//! Letters beyond plain English take more than one byte: "é" is 2, "世" is 3,
//! "👋" is 4. The first byte says how many are coming; the rest all look like
//! `10xxxxxx` (between 0x80 and 0xBF) and are called "continuation" bytes.
//!
//! The shell's output arrives in random-sized pieces, so a letter can be cut in
//! half between two reads. This little helper keeps the first half until the
//! rest arrives.

/// The mark terminals show for bytes that aren't a valid letter: �
pub(super) const REPLACEMENT: char = '\u{FFFD}';

/// A letter that's still being put together.
#[derive(Debug, Default)]
pub(super) struct Utf8 {
    bytes: [u8; 4],
    /// How many bytes we have so far (0 = not in the middle of a letter).
    have: usize,
    /// How many bytes this letter needs in total.
    need: usize,
}

/// Is this one of the "more of the same letter" bytes?
pub(super) fn is_continuation(byte: u8) -> bool {
    (0x80..=0xBF).contains(&byte)
}

impl Utf8 {
    /// True while we're holding the first part of a letter.
    pub(super) fn is_waiting(&self) -> bool {
        self.have > 0
    }

    /// Forgets a half-finished letter.
    pub(super) fn reset(&mut self) {
        self.have = 0;
        self.need = 0;
    }

    /// Takes one byte of 0x80 or above. Gives back a letter when one is complete,
    /// or `None` while it still needs more bytes.
    ///
    /// The caller must only pass a continuation byte while `is_waiting()`; any
    /// other byte means the letter was broken off, which the caller handles.
    pub(super) fn push(&mut self, byte: u8) -> Option<char> {
        if !self.is_waiting() {
            // The first byte tells us the length. Anything else here (a stray
            // continuation byte, or a byte UTF-8 never uses) is broken.
            self.need = match byte {
                0xC2..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF4 => 4,
                _ => return Some(REPLACEMENT),
            };
        }
        self.bytes[self.have] = byte;
        self.have += 1;
        if self.have < self.need {
            return None;
        }
        // All bytes are here. Rust's own UTF-8 check also rejects the sneaky
        // invalid ones (like "surrogates", or a letter written the long way).
        let letter = std::str::from_utf8(&self.bytes[..self.need])
            .ok()
            .and_then(|text| text.chars().next())
            .unwrap_or(REPLACEMENT);
        self.reset();
        Some(letter)
    }
}
