//! "Did you mean...?" — finds the known phrase closest to a typo.

/// The start of every phrase Noob understands.
pub const PHRASES: [&str; 14] = [
    "where am i",
    "show files",
    "make folder",
    "make file",
    "read",
    "copy",
    "move",
    "rename",
    "delete",
    "clear",
    "help",
    "bye",
    "exit",
    "quit",
];

/// A typo counts as "close" if it needs at most this many letter changes.
const MAX_TYPOS: usize = 2;

/// Returns the known phrase that best matches the start of `input`, if one is close enough.
pub fn closest(input: &str) -> Option<&'static str> {
    let words: Vec<String> = input.split_whitespace().map(str::to_lowercase).collect();

    PHRASES
        .iter()
        .map(|&phrase| {
            // Compare "make folder" with the first two typed words, "read" with the first one, ...
            let phrase_length = phrase.split_whitespace().count();
            let start_of_input = words
                .iter()
                .take(phrase_length)
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(" ");
            (edit_distance(&start_of_input, phrase), phrase)
        })
        .filter(|&(typos, _)| typos <= MAX_TYPOS)
        .min_by_key(|&(typos, _)| typos)
        .map(|(_, phrase)| phrase)
}

/// How many single-letter changes turn `a` into `b`.
///
/// This is the classic "Levenshtein distance". We fill a table row by row:
/// `previous[j]` is the cost of turning the letters of `a` seen so far
/// into the first `j` letters of `b`.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();

    for (i, a_letter) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, &b_letter) in b.iter().enumerate() {
            let swap_cost = if a_letter == b_letter { 0 } else { 1 };
            let swap = previous[j] + swap_cost;
            let delete = previous[j + 1] + 1;
            let insert = current[j] + 1;
            current.push(swap.min(delete).min(insert));
        }
        previous = current;
    }
    previous[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_letter_changes() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("same", "same"), 0);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    #[test]
    fn suggests_the_phrase_for_a_typo() {
        assert_eq!(closest("maek folder photos"), Some("make folder"));
        assert_eq!(closest("shwo files"), Some("show files"));
        assert_eq!(closest("delet notes.txt"), Some("delete"));
    }

    #[test]
    fn ignores_capital_letters() {
        assert_eq!(closest("SHWO FILES"), Some("show files"));
    }

    #[test]
    fn suggests_nothing_for_nonsense() {
        assert_eq!(closest("banana bread"), None);
    }
}
