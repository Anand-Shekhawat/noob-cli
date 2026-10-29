//! The text shown by `help` and `help <command>`, built from the command table.

use crate::parser;
use crate::table::{self, COMMANDS, Group};

/// Width of the example column, so the summaries line up.
const EXAMPLE_WIDTH: usize = 30;

/// The whole list, grouped.
pub fn everything() -> String {
    let mut lines = vec!["Here's everything Noob understands:".to_string()];
    for group in Group::ALL {
        lines.push(String::new());
        lines.push(group.title().to_string());
        for spec in COMMANDS.iter().filter(|spec| spec.group == group) {
            lines.push(format!(
                "  {:<EXAMPLE_WIDTH$} {}",
                spec.example, spec.summary
            ));
        }
    }
    lines.push(String::new());
    lines.push("Tip: put names with spaces in quotes, like: make folder \"my photos\"".to_string());
    lines.push("For more about one command, type help and its name, like: help copy".to_string());
    lines.join("\n")
}

/// More about one command, like `help copy`. `None` if there's no such command.
pub fn about(topic: &str) -> Option<String> {
    let spec = table::find(topic)?;
    let mut lines = vec![
        format!("{}: {}", spec.main_phrase(), spec.summary),
        format!("  Example:       {}", spec.example),
    ];
    // Run the example through the parser to show its real command, so they can never disagree.
    let real = parser::parse(spec.example)
        .ok()
        .and_then(|command| command.real_command());
    if let Some(real) = real {
        lines.push(format!("  Real command:  {real}"));
    }
    if let [_, others @ ..] = spec.phrases
        && !others.is_empty()
    {
        lines.push(format!("  Also works:    {}", others.join(", ")));
    }
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_every_command_under_its_group() {
        let help = everything();
        for spec in COMMANDS {
            assert!(help.contains(spec.example), "{} is missing", spec.example);
        }
        for group in Group::ALL {
            assert!(help.contains(group.title()));
        }
    }

    #[test]
    fn explains_one_command_with_its_real_command_and_other_names() {
        let help = about("make dir").unwrap();
        assert!(help.starts_with("make folder: make a new folder"));
        assert!(help.contains("Real command:  mkdir -p photos"));
        assert!(help.contains("Also works:    make dir, new folder, create folder"));
    }

    #[test]
    fn says_so_when_there_is_no_help_for_a_topic() {
        assert!(about("banana").is_none());
    }
}
