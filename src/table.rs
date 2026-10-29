//! The command table: every phrase Noob understands, written down once.
//!
//! The parser, the "did you mean" helper, Tab completion and `help` all read
//! this one list. Adding a command means adding one entry here, plus its real
//! work in `actions` (the compiler reminds you about that part).

use crate::command::Command;

/// What comes after the phrase.
pub enum Shape {
    /// Nothing: `show files`
    Nothing,
    /// One name: `make folder photos`
    One,
    /// A name you can leave out: `help` or `help copy`
    Optional,
    /// Two names with a joining word between them: `copy a.txt to b.txt`
    Two { joiners: &'static [&'static str] },
}

/// Sections of the help list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Group {
    GettingAround,
    Looking,
    Making,
    Moving,
    Finding,
    Other,
}

impl Group {
    pub const ALL: [Group; 6] = [
        Group::GettingAround,
        Group::Looking,
        Group::Making,
        Group::Moving,
        Group::Finding,
        Group::Other,
    ];

    pub fn title(self) -> &'static str {
        match self {
            Group::GettingAround => "Getting around",
            Group::Looking => "Looking at things",
            Group::Making => "Making and changing",
            Group::Moving => "Copying, moving, deleting",
            Group::Finding => "Finding",
            Group::Other => "Other",
        }
    }
}

/// One command: how to say it, how to explain it, and how to build it.
pub struct Spec {
    /// Ways to say it. The first one is the main one, shown in help.
    pub phrases: &'static [&'static str],
    pub shape: Shape,
    /// A full example. Tests check that every example really works.
    pub example: &'static str,
    pub summary: &'static str,
    pub group: Group,
    /// Turns the names that were typed into a `Command`.
    pub build: fn(Vec<String>) -> Command,
}

impl Spec {
    pub fn main_phrase(&self) -> &'static str {
        self.phrases.first().copied().unwrap_or_default()
    }
}

// Small helpers that pick names out of the list the parser hands to `build`.
fn one(names: Vec<String>) -> String {
    names.into_iter().next().unwrap_or_default()
}

fn optional(names: Vec<String>) -> Option<String> {
    names.into_iter().next()
}

fn two(names: Vec<String>) -> (String, String) {
    let mut names = names.into_iter();
    let first = names.next().unwrap_or_default();
    let second = names.next().unwrap_or_default();
    (first, second)
}

/// "~" means your home folder, like in a real terminal: "~/photos" is the photos folder in it.
fn expand_home(name: String) -> String {
    expand_home_with(name, std::env::home_dir().as_deref())
}

fn expand_home_with(name: String, home: Option<&std::path::Path>) -> String {
    let Some(home) = home else {
        return name;
    };
    if name == "~" {
        return home.display().to_string();
    }
    match name.strip_prefix("~/") {
        Some(rest) => home.join(rest).display().to_string(),
        None => name,
    }
}

// Names that are places (files and folders) get "~" expanded. Text, like in
// `write "~ is my favourite letter" to notes.txt`, is left exactly as typed.
fn place(names: Vec<String>) -> String {
    expand_home(one(names))
}

fn optional_place(names: Vec<String>) -> Option<String> {
    optional(names).map(expand_home)
}

fn two_places(names: Vec<String>) -> (String, String) {
    let (first, second) = two(names);
    (expand_home(first), expand_home(second))
}

fn text_and_place(names: Vec<String>) -> (String, String) {
    let (text, place) = two(names);
    (text, expand_home(place))
}

/// Every command Noob understands. `|names| ...` is a closure: a tiny unnamed function.
pub static COMMANDS: &[Spec] = &[
    // Getting around
    Spec {
        phrases: &["where am i"],
        shape: Shape::Nothing,
        example: "where am i",
        summary: "show which folder you're in",
        group: Group::GettingAround,
        build: |_| Command::WhereAmI,
    },
    Spec {
        phrases: &["go into", "go to"],
        shape: Shape::One,
        example: "go into photos",
        summary: "go inside a folder (in the noob> prompt)",
        group: Group::GettingAround,
        build: |names| Command::GoInto(place(names)),
    },
    Spec {
        phrases: &["go back", "go up"],
        shape: Shape::Nothing,
        example: "go back",
        summary: "go up to the folder this one is in",
        group: Group::GettingAround,
        build: |_| Command::GoBack,
    },
    Spec {
        phrases: &["go home"],
        shape: Shape::Nothing,
        example: "go home",
        summary: "go to your home folder (~)",
        group: Group::GettingAround,
        build: |_| Command::GoHome,
    },
    // Looking at things
    Spec {
        phrases: &["show files", "list files"],
        shape: Shape::Optional,
        example: "show files",
        summary: "list the files and folders here (or in a folder you name)",
        group: Group::Looking,
        build: |names| Command::ShowFiles(optional_place(names)),
    },
    Spec {
        phrases: &["show all files", "show hidden files"],
        shape: Shape::Optional,
        example: "show all files",
        summary: "also list hidden files (names starting with .)",
        group: Group::Looking,
        build: |names| Command::ShowAllFiles(optional_place(names)),
    },
    Spec {
        phrases: &["show details"],
        shape: Shape::Optional,
        example: "show details",
        summary: "list with sizes, and when each thing last changed",
        group: Group::Looking,
        build: |names| Command::ShowDetails(optional_place(names)),
    },
    Spec {
        phrases: &["show tree"],
        shape: Shape::Optional,
        example: "show tree",
        summary: "draw the folders inside folders, as a tree",
        group: Group::Looking,
        build: |names| Command::ShowTree(optional_place(names)),
    },
    Spec {
        phrases: &["show size", "how big is"],
        shape: Shape::One,
        example: "show size photos",
        summary: "how much space a file or folder takes up",
        group: Group::Looking,
        build: |names| Command::ShowSize(place(names)),
    },
    Spec {
        phrases: &["read"],
        shape: Shape::One,
        example: "read notes.txt",
        summary: "show what's inside a file",
        group: Group::Looking,
        build: |names| Command::Read(place(names)),
    },
    // Making and changing
    Spec {
        phrases: &["make folder", "make dir", "new folder", "create folder"],
        shape: Shape::One,
        example: "make folder photos",
        summary: "make a new folder",
        group: Group::Making,
        build: |names| Command::MakeFolder(place(names)),
    },
    Spec {
        phrases: &["make file", "new file", "create file"],
        shape: Shape::One,
        example: "make file notes.txt",
        summary: "make a new, empty file",
        group: Group::Making,
        build: |names| Command::MakeFile(place(names)),
    },
    Spec {
        phrases: &["write"],
        shape: Shape::Two {
            joiners: &["to", "into"],
        },
        example: "write \"hello\" to notes.txt",
        summary: "replace a file's text (asks first if it has some)",
        group: Group::Making,
        build: |names| {
            let (text, file) = text_and_place(names);
            Command::Write { text, file }
        },
    },
    Spec {
        phrases: &["add"],
        shape: Shape::Two { joiners: &["to"] },
        example: "add \"buy milk\" to list.txt",
        summary: "add a line to the end of a file",
        group: Group::Making,
        build: |names| {
            let (text, file) = text_and_place(names);
            Command::Add { text, file }
        },
    },
    Spec {
        phrases: &["open"],
        shape: Shape::One,
        example: "open notes.txt",
        summary: "open a file or folder in its usual app",
        group: Group::Making,
        build: |names| Command::Open(place(names)),
    },
    // Copying, moving, deleting
    Spec {
        phrases: &["copy"],
        shape: Shape::Two {
            joiners: &["to", "into"],
        },
        example: "copy a.txt to b.txt",
        summary: "copy a file or folder (to a new name, or into a folder)",
        group: Group::Moving,
        build: |names| {
            let (from, to) = two_places(names);
            Command::Copy { from, to }
        },
    },
    Spec {
        phrases: &["move"],
        shape: Shape::Two {
            joiners: &["into", "to"],
        },
        example: "move a.txt into docs",
        summary: "move a file or folder somewhere else",
        group: Group::Moving,
        build: |names| {
            let (from, to) = two_places(names);
            Command::Move { from, to }
        },
    },
    Spec {
        phrases: &["rename"],
        shape: Shape::Two { joiners: &["to"] },
        example: "rename old.txt to new.txt",
        summary: "give a file or folder a new name",
        group: Group::Moving,
        build: |names| {
            let (from, to) = two_places(names);
            Command::Rename { from, to }
        },
    },
    Spec {
        phrases: &["delete", "remove"],
        shape: Shape::One,
        example: "delete notes.txt",
        summary: "move a file or folder to the Trash (asks first)",
        group: Group::Moving,
        build: |names| Command::Delete(place(names)),
    },
    // Finding
    Spec {
        phrases: &["find file", "find folder"],
        shape: Shape::One,
        example: "find file cat",
        summary: "find things by name, here and in every folder inside",
        group: Group::Finding,
        build: |names| Command::FindFile(one(names)),
    },
    Spec {
        phrases: &["find", "search"],
        shape: Shape::Two { joiners: &["in"] },
        example: "find \"hello\" in notes.txt",
        summary: "find text inside a file, or inside every file in a folder",
        group: Group::Finding,
        build: |names| {
            let (text, place) = text_and_place(names);
            Command::FindText { text, place }
        },
    },
    // Other
    Spec {
        phrases: &["clear"],
        shape: Shape::Nothing,
        example: "clear",
        summary: "clear the screen",
        group: Group::Other,
        build: |_| Command::Clear,
    },
    Spec {
        phrases: &["help", "-h", "--help"],
        shape: Shape::Optional,
        example: "help copy",
        summary: "show this list, or more about one command",
        group: Group::Other,
        build: |names| Command::Help(optional(names)),
    },
    Spec {
        phrases: &["bye", "exit", "quit"],
        shape: Shape::Nothing,
        example: "bye",
        summary: "leave Noob",
        group: Group::Other,
        build: |_| Command::Bye,
    },
];

/// Every way of saying every command, main phrases first within each command.
pub fn all_phrases() -> impl Iterator<Item = &'static str> {
    COMMANDS
        .iter()
        .flat_map(|spec| spec.phrases.iter().copied())
}

/// The command a help topic is about: "copy", "make folder", or just "make".
pub fn find(topic: &str) -> Option<&'static Spec> {
    let topic = topic.trim().to_lowercase();
    let exact = COMMANDS
        .iter()
        .find(|spec| spec.phrases.contains(&topic.as_str()));
    exact.or_else(|| {
        COMMANDS
            .iter()
            .find(|spec| spec.main_phrase().starts_with(&topic))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use std::collections::HashSet;

    #[test]
    fn every_example_runs_its_own_command() {
        for spec in COMMANDS {
            let matched = parser::spec_for(spec.example)
                .unwrap_or_else(|| panic!("example {:?} isn't understood", spec.example));
            assert!(
                std::ptr::eq(matched, spec),
                "example {:?} runs a different command",
                spec.example
            );
            assert!(parser::parse(spec.example).is_ok(), "{}", spec.example);
        }
    }

    #[test]
    fn no_phrase_is_used_twice() {
        let mut seen = HashSet::new();
        for phrase in all_phrases() {
            assert!(seen.insert(phrase), "{phrase:?} is in the table twice");
        }
    }

    #[test]
    fn phrases_are_lowercase_and_tidy() {
        for phrase in all_phrases() {
            assert_eq!(phrase, phrase.to_lowercase(), "{phrase:?} has capitals");
            assert_eq!(phrase, phrase.trim(), "{phrase:?} has extra spaces");
        }
    }

    #[test]
    fn every_command_has_a_phrase_and_every_group_has_a_command() {
        assert!(COMMANDS.iter().all(|spec| !spec.phrases.is_empty()));
        for group in Group::ALL {
            assert!(COMMANDS.iter().any(|spec| spec.group == group), "{group:?}");
        }
    }

    #[test]
    fn a_squiggle_means_the_home_folder() {
        let home = Some(std::path::Path::new("/Users/sam"));
        let expand = |name: &str| expand_home_with(name.to_string(), home);
        assert_eq!(expand("~"), "/Users/sam");
        assert_eq!(expand("~/photos"), "/Users/sam/photos");
        assert_eq!(expand("~bob"), "~bob", "only ~ on its own or ~/ means home");
        assert_eq!(expand("a~b"), "a~b");
    }

    #[test]
    fn finds_help_topics_by_any_name_or_a_start() {
        assert_eq!(find("copy").map(Spec::main_phrase), Some("copy"));
        assert_eq!(find("make dir").map(Spec::main_phrase), Some("make folder"));
        assert_eq!(find("MAKE").map(Spec::main_phrase), Some("make folder"));
        assert!(find("banana").is_none());
    }
}
