//! Helpers shared by the tests in this folder.

use std::fs;
use std::path::{Path, PathBuf};

/// A fresh, empty folder for one test. It deletes itself when the test ends,
/// because Rust runs `drop` automatically when a value goes out of scope.
pub struct TestFolder(PathBuf);

impl TestFolder {
    /// `test_name` must be different for every test, because tests run at the same time.
    pub fn new(test_name: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("noob-test-{test_name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path); // clean up leftovers from a crashed run
        fs::create_dir_all(&path).expect("create test folder");
        TestFolder(path)
    }

    /// The test folder itself.
    pub fn dir(&self) -> &Path {
        &self.0
    }

    /// The test folder's full path, as text.
    pub fn path(&self) -> String {
        self.0.display().to_string()
    }

    /// The full path of `name` inside this test folder, as text.
    pub fn at(&self, name: &str) -> String {
        self.0.join(name).display().to_string()
    }
}

impl Drop for TestFolder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Makes a "named pipe": a special file that waits forever for someone to write into it.
/// Programs that read it without checking first get stuck.
#[cfg(unix)]
pub fn make_pipe(test: &TestFolder, name: &str) -> String {
    let path = test.at(name);
    let made = std::process::Command::new("mkfifo")
        .arg(&path)
        .status()
        .expect("run mkfifo");
    assert!(made.success(), "mkfifo failed");
    path
}
