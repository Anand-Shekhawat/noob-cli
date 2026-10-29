//! Turning numbers into friendly words: "1.5 KB", "3 days ago", "1 file".

use std::time::Duration;

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
const MONTH: u64 = 30 * DAY;
const YEAR: u64 = 365 * DAY;

/// "1 file", "2 files".
pub(super) fn plural(count: u64, word: &str) -> String {
    if count == 1 {
        format!("1 {word}")
    } else {
        format!("{count} {word}s")
    }
}

/// 1536 bytes is "1.5 KB". Each step up is 1024 times bigger.
pub(super) fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return plural(bytes, "byte");
    }
    let mut size = bytes as f64 / 1024.0;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{size:.1} {}", UNITS[unit])
}

/// How long ago something happened, roughly: "just now", "5 minutes ago", "2 years ago".
pub(super) fn human_age(age: Duration) -> String {
    let seconds = age.as_secs();
    let (count, unit) = match seconds {
        s if s < MINUTE => return "just now".to_string(),
        s if s < HOUR => (s / MINUTE, "minute"),
        s if s < DAY => (s / HOUR, "hour"),
        s if s < MONTH => (s / DAY, "day"),
        s if s < YEAR => (s / MONTH, "month"),
        s => (s / YEAR, "year"),
    };
    format!("{} ago", plural(count, unit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_read_like_a_person_would_say_them() {
        assert_eq!(human_size(0), "0 bytes");
        assert_eq!(human_size(1), "1 byte");
        assert_eq!(human_size(1536), "1.5 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
    }

    #[test]
    fn ages_are_rounded_to_the_biggest_unit() {
        assert_eq!(human_age(Duration::from_secs(30)), "just now");
        assert_eq!(human_age(Duration::from_secs(60)), "1 minute ago");
        assert_eq!(human_age(Duration::from_secs(2 * HOUR + 5)), "2 hours ago");
        assert_eq!(human_age(Duration::from_secs(3 * DAY)), "3 days ago");
        assert_eq!(human_age(Duration::from_secs(2 * YEAR)), "2 years ago");
    }
}
