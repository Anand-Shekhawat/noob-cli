//! How many cells a letter takes on screen: 0, 1 or 2.
//!
//! Most letters take one cell. Chinese, Japanese and Korean letters and most
//! emoji are drawn twice as wide, so they take two. A few take none: "combining"
//! marks that sit on top of the letter before them (like the accent in
//! e + ◌́ = é), and invisible joiners.
//!
//! The official list is Unicode's "East Asian Width" table. We don't add a
//! crate for it; the ranges below cover the letters people actually meet in a
//! terminal. A letter we get wrong just looks a little squashed or spaced out.

/// Ranges of letters that take two cells.
const WIDE: &[(u32, u32)] = &[
    (0x1100, 0x115F),   // Korean Hangul starting sounds
    (0x231A, 0x231B),   // ⌚ ⌛
    (0x2329, 0x232A),   // 〈 〉
    (0x23E9, 0x23EC),   // ⏩ … ⏬
    (0x23F0, 0x23F0),   // ⏰
    (0x23F3, 0x23F3),   // ⏳
    (0x25FD, 0x25FE),   // ◽ ◾
    (0x2614, 0x2615),   // ☔ ☕
    (0x2648, 0x2653),   // ♈ … ♓ star signs
    (0x267F, 0x267F),   // ♿
    (0x2693, 0x2693),   // ⚓
    (0x26A1, 0x26A1),   // ⚡
    (0x26AA, 0x26AB),   // ⚪ ⚫
    (0x26BD, 0x26BE),   // ⚽ ⚾
    (0x26C4, 0x26C5),   // ⛄ ⛅
    (0x26CE, 0x26CE),   // ⛎
    (0x26D4, 0x26D4),   // ⛔
    (0x26EA, 0x26EA),   // ⛪
    (0x26F2, 0x26F3),   // ⛲ ⛳
    (0x26F5, 0x26F5),   // ⛵
    (0x26FA, 0x26FA),   // ⛺
    (0x26FD, 0x26FD),   // ⛽
    (0x2705, 0x2705),   // ✅
    (0x270A, 0x270B),   // ✊ ✋
    (0x2728, 0x2728),   // ✨
    (0x274C, 0x274C),   // ❌
    (0x274E, 0x274E),   // ❎
    (0x2753, 0x2755),   // ❓ ❔ ❕
    (0x2757, 0x2757),   // ❗
    (0x2795, 0x2797),   // ➕ ➖ ➗
    (0x27B0, 0x27B0),   // ➰
    (0x27BF, 0x27BF),   // ➿
    (0x2B1B, 0x2B1C),   // ⬛ ⬜
    (0x2B50, 0x2B50),   // ⭐
    (0x2B55, 0x2B55),   // ⭕
    (0x2E80, 0x303E),   // CJK radicals, symbols and punctuation
    (0x3041, 0x33FF),   // Japanese kana, CJK compatibility
    (0x3400, 0x4DBF),   // CJK extension A
    (0x4E00, 0x9FFF),   // the main block of Chinese characters
    (0xA000, 0xA4CF),   // Yi
    (0xA960, 0xA97F),   // more Hangul
    (0xAC00, 0xD7A3),   // Korean Hangul syllables
    (0xF900, 0xFAFF),   // CJK compatibility letters
    (0xFE10, 0xFE19),   // vertical punctuation
    (0xFE30, 0xFE6F),   // CJK compatibility forms
    (0xFF00, 0xFF60),   // full-width Latin letters and punctuation
    (0xFFE0, 0xFFE6),   // full-width signs (￥ and friends)
    (0x16FE0, 0x16FE4), // ideographic symbols
    (0x17000, 0x18AFF), // Tangut
    (0x1B000, 0x1B2FF), // more kana
    (0x1F004, 0x1F004), // 🀄
    (0x1F0CF, 0x1F0CF), // 🃏
    (0x1F18E, 0x1F18E), // 🆎
    (0x1F191, 0x1F19A), // 🆑 … 🆚
    (0x1F200, 0x1F251), // enclosed CJK
    (0x1F300, 0x1F64F), // pictures and smileys 🌍 😀
    (0x1F680, 0x1F6FF), // transport and map symbols 🚀
    (0x1F7E0, 0x1F7EB), // coloured circles and squares 🟠
    (0x1F90C, 0x1F9FF), // more pictures and smileys 🤖
    (0x1FA70, 0x1FAFF), // and more 🪐
    (0x20000, 0x2FFFD), // rare Chinese characters
    (0x30000, 0x3FFFD), // and rarer ones
];

/// Ranges of letters that take no room of their own.
const ZERO: &[(u32, u32)] = &[
    (0x0300, 0x036F),   // accents that combine with the letter before
    (0x0483, 0x0489),   // Cyrillic marks
    (0x0591, 0x05BD),   // Hebrew marks
    (0x0610, 0x061A),   // Arabic marks
    (0x064B, 0x065F),   // Arabic vowel marks
    (0x1AB0, 0x1AFF),   // more combining marks
    (0x1DC0, 0x1DFF),   // and more
    (0x200B, 0x200F),   // invisible spaces and joiners (the emoji "glue" ZWJ)
    (0x2028, 0x202E),   // invisible direction marks
    (0x2060, 0x2064),   // invisible word joiners
    (0x20D0, 0x20FF),   // marks for symbols
    (0xFE00, 0xFE0F),   // "variation selectors" (draw as emoji / as text)
    (0xFE20, 0xFE2F),   // half marks
    (0xFEFF, 0xFEFF),   // the invisible "byte order mark"
    (0x1F3FB, 0x1F3FF), // skin-tone changers for emoji 👋🏽
    (0xE0000, 0xE0FFF), // invisible tags
];

/// How many cells `letter` takes: 0, 1 or 2.
pub(super) fn letter_width(letter: char) -> usize {
    let code = u32::from(letter);
    // Plain English letters are by far the most common: answer them quickly.
    if code < 0x300 {
        return 1;
    }
    if in_ranges(code, ZERO) {
        0
    } else if in_ranges(code, WIDE) {
        2
    } else {
        1
    }
}

/// Is `code` inside one of the (sorted) `ranges`? A "binary search" halves the
/// list each step instead of checking every range.
fn in_ranges(code: u32, ranges: &[(u32, u32)]) -> bool {
    ranges
        .binary_search_by(|&(first, last)| {
            if last < code {
                std::cmp::Ordering::Less
            } else if first > code {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_have_the_width_terminals_give_them() {
        assert_eq!(letter_width('a'), 1);
        assert_eq!(letter_width('é'), 1);
        assert_eq!(letter_width('➜'), 1);
        assert_eq!(letter_width('世'), 2);
        assert_eq!(letter_width('한'), 2);
        assert_eq!(letter_width('👋'), 2);
        assert_eq!(letter_width('🚀'), 2);
        assert_eq!(letter_width('\u{301}'), 0); // combining accent
        assert_eq!(letter_width('\u{200D}'), 0); // emoji glue
        assert_eq!(letter_width('\u{1F3FD}'), 0); // skin tone, inside the 🌍 block
    }

    #[test]
    fn the_range_lists_are_sorted_so_the_search_works() {
        for ranges in [WIDE, ZERO] {
            for pair in ranges.windows(2) {
                assert!(pair[0].0 <= pair[0].1 && pair[0].1 < pair[1].0);
            }
        }
    }
}
