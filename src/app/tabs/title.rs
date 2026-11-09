//! A short name for a tab, made from the title the shell sets.
//!
//! zsh and bash usually set the title to `you@computer:~/Documents/photos`.
//! That's too long for a tab, so we show just the last folder: `photos`.

/// What to show on a tab. `None` (no title yet) shows "Shell".
pub fn short_title(title: Option<&str>) -> String {
    let Some(title) = title.map(str::trim).filter(|title| !title.is_empty()) else {
        return "Shell".to_string();
    };
    match title.split_once(':') {
        // "you@computer:folder": the shell's usual title.
        Some((who, folder)) if who.contains('@') => last_folder(folder.trim()),
        // Anything else (like "nano notes.txt") is shown as the program set it.
        _ => title.to_string(),
    }
}

/// `~/Documents/photos` → `photos`. `~` and `/` stay as they are.
fn last_folder(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    match trimmed.rsplit('/').next() {
        Some(last) if !last.is_empty() => last.to_string(),
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_title_becomes_its_last_folder() {
        assert_eq!(short_title(Some("anand@mac:~/Documents/photos")), "photos");
        assert_eq!(short_title(Some("anand@mac:/usr/local/bin")), "bin");
        assert_eq!(short_title(Some("anand@mac:~/photos/")), "photos");
    }

    #[test]
    fn the_home_folder_and_the_top_folder_keep_their_symbols() {
        assert_eq!(short_title(Some("anand@mac:~")), "~");
        assert_eq!(short_title(Some("anand@mac:/")), "/");
    }

    #[test]
    fn a_program_title_is_kept_as_it_is() {
        assert_eq!(short_title(Some("nano notes.txt")), "nano notes.txt");
        assert_eq!(short_title(Some("top")), "top");
    }

    #[test]
    fn no_title_yet_shows_shell() {
        assert_eq!(short_title(None), "Shell");
        assert_eq!(short_title(Some("   ")), "Shell");
    }
}
