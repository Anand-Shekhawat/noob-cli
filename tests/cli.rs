//! Tests that run the real `noob` program, the way a person would.
//!
//! Each test gets its own empty "playground" folder to work in, and its own pretend
//! home folder, so your real files and your real `~/.noob_history` are never touched.
//! (No test answers "yes" to a delete, so nothing ever goes to your real Trash.)

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

struct Playground {
    root: PathBuf,
}

impl Playground {
    fn new(test_name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("noob-cli-test-{test_name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root); // clean up leftovers from a crashed run
        fs::create_dir_all(root.join("work")).expect("create work folder");
        fs::create_dir_all(root.join("home")).expect("create home folder");
        Playground { root }
    }

    /// Where noob runs. Its full, standard path, because that's what noob reports.
    fn work(&self) -> PathBuf {
        fs::canonicalize(self.root.join("work")).expect("work folder")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_noob"));
        command
            .current_dir(self.work())
            .env("HOME", self.home())
            .env("NO_COLOR", "1");
        command
    }

    /// Like typing `noob make folder photos` in the normal terminal.
    fn run(&self, words: &[&str]) -> Output {
        self.command().args(words).output().expect("run noob")
    }

    /// Opens the `noob>` prompt and types each line in `typed`.
    fn prompt(&self, typed: &str) -> Output {
        let mut child = self
            .command()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("start noob");
        let mut keyboard = child.stdin.take().expect("keyboard");
        keyboard.write_all(typed.as_bytes()).expect("type");
        drop(keyboard); // closing the keyboard is like pressing Ctrl+D
        child.wait_with_output().expect("noob finishes")
    }
}

impl Drop for Playground {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

#[test]
fn makes_a_folder_from_the_normal_terminal() {
    let playground = Playground::new("make-folder");
    let output = playground.run(&["make", "folder", "my photos"]);

    assert!(output.status.success());
    assert!(playground.work().join("my photos").is_dir());
    let shown = text(&output.stdout);
    assert!(shown.contains("✓ Made folder \"my photos\""), "{shown}");
    assert!(
        shown.contains("(real command: mkdir -p 'my photos')"),
        "{shown}"
    );
}

#[test]
fn unknown_phrases_fail_with_a_suggestion() {
    let playground = Playground::new("unknown");
    let output = playground.run(&["maek", "folder", "x"]);

    assert!(!output.status.success(), "scripts can tell it failed");
    assert!(text(&output.stderr).contains("Did you mean \"make folder\"?"));
}

#[test]
fn moving_around_explains_it_needs_the_prompt() {
    let playground = Playground::new("go-outside");
    let output = playground.run(&["go", "home"]);

    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("only works inside the noob prompt"));
}

#[test]
fn the_prompt_goes_into_folders_and_back() {
    let playground = Playground::new("go");
    let output =
        playground.prompt("make folder a\ngo into a\nwhere am i\ngo back\nwhere am i\nbye\n");

    assert!(output.status.success());
    let shown = text(&output.stdout);
    let inside = format!("You are in {}", playground.work().join("a").display());
    let back = format!("You are in {}\n", playground.work().display());
    assert!(shown.contains(&inside), "{shown}");
    assert!(shown.contains(&back), "{shown}");
}

#[test]
fn quoted_names_can_hold_spaces_and_the_word_to() {
    let playground = Playground::new("quotes");
    let output = playground.prompt("make folder docs\nmake file \"notes to self.txt\"\ncopy \"notes to self.txt\" to docs\nbye\n");

    assert!(output.status.success(), "{}", text(&output.stderr));
    assert!(playground.work().join("docs/notes to self.txt").is_file());
}

#[test]
fn delete_asks_first_and_anything_but_yes_keeps_the_file() {
    let playground = Playground::new("delete-no");
    fs::write(playground.work().join("keep.txt"), "precious").unwrap();
    let output = playground.prompt("delete keep.txt\nno\nbye\n");

    assert!(text(&output.stdout).contains("Move \"keep.txt\" to the Trash?"));
    assert!(text(&output.stdout).contains("Okay, I left everything as it was."));
    assert_eq!(
        fs::read_to_string(playground.work().join("keep.txt")).unwrap(),
        "precious"
    );
}

#[test]
fn history_is_saved_in_the_home_folder() {
    let playground = Playground::new("history");
    playground.prompt("show files\nbye\n");

    let history = fs::read_to_string(playground.home().join(".noob_history")).unwrap();
    assert!(history.contains("show files"), "{history}");
}

#[test]
fn no_colour_codes_when_the_output_is_not_a_terminal() {
    let playground = Playground::new("colours");
    let output = playground
        .command()
        .env_remove("NO_COLOR")
        .args(["help"])
        .output()
        .unwrap();

    assert!(
        !text(&output.stdout).contains('\x1b'),
        "colour codes leaked into a file"
    );
}

#[test]
fn help_lists_every_group() {
    let playground = Playground::new("help");
    let shown = text(&playground.run(&["help"]).stdout);
    for group in [
        "Getting around",
        "Looking at things",
        "Making and changing",
        "Finding",
    ] {
        assert!(shown.contains(group), "{group} missing from:\n{shown}");
    }
}

#[test]
fn closing_the_keyboard_leaves_quietly() {
    let playground = Playground::new("eof");
    let output = playground.prompt("");
    assert!(output.status.success());
}

#[test]
fn finds_text_and_names_from_the_normal_terminal() {
    let playground = Playground::new("find");
    let notes = playground.work().join("notes");
    fs::create_dir(&notes).unwrap();
    fs::write(notes.join("todo.txt"), "buy milk\ncall Sam\n").unwrap();

    let by_name = text(&playground.run(&["find", "file", "todo"]).stdout);
    assert!(by_name.contains("notes/todo.txt"), "{by_name}");
    let by_text = text(&playground.run(&["find", "sam", "in", "notes"]).stdout);
    assert!(by_text.contains("notes/todo.txt:2  call Sam"), "{by_text}");
}
