# Rust review v1 - Noob CLI

Checks: clippy `--all-targets -D warnings` clean, `cargo fmt --check` clean, 36 tests pass.
No `unsafe`, no `unwrap` in production paths, no dependencies. Verdict: WARNING (no CRITICAL, no data-loss path found in normal use).
Items marked "verified" were reproduced with the built binary in a scratch folder.

## CRITICAL
None.

## HIGH
None. (The delete-a-symlink-to-a-folder behaviour is correct, verified: the link is removed, the target's contents survive. It is just untested, see M5.)

## MEDIUM

### M1. Dangling symlinks defeat the "never overwrite" check - actions.rs:213-234, 139, 186
`Path::exists()` follows symlinks, so a broken link counts as "does not exist".
- Verified: `copy a.txt to dang` (dang -> real/gone.txt) silently created `real/gone.txt` (wrote through the link).
- Verified: `move a.txt to dang2` (dang2 -> nowhere) silently replaced the link with the file.
Fix (readable): use `fs::symlink_metadata(path).is_ok()` in a small helper `fn taken(path: &Path) -> bool` and use it in `free_destination`, `rename`, `make_folder`.
For copy, a race-free alternative: open the target with `OpenOptions::new().write(true).create_new(true)` and `io::copy` into it (`create_new` never follows a link and never clobbers). Trade-off: permission bits are not copied.

### M2. Error names the wrong path - actions.rs:180, 189 (also 173)
`move`/`rename` map every error to `from`. Verified: `rename b.txt to sub/x` (no `sub` folder) prints `I couldn't find "b.txt"` although b.txt exists. Same for `move` into a missing parent; `copy` blames `to` even when the source failed. Fix: the OS error does not say which path; check the destination's parent exists first (`ActionError::NotFound(parent)`), or word the NotFound arm for these actions neutrally ("a folder in that path is missing").

### M3. Raw OS text leaks through `Other` - actions.rs:75, 62
Verified: `move real2 into real2` -> `Something went wrong: Invalid argument (os error 22)`. Also likely: cross-device move (`ErrorKind::CrossesDevices`), `NotADirectory`, `IsADirectory`. Add explicit arms (e.g. `CrossesDevices` -> "can't move across drives yet", `DirectoryNotEmpty` -> `FolderNotEmpty`) and a test for `friendly`. Also guard "move a folder into itself" before calling rename.

### M4. `delete` pre-check is redundant and racy - actions.rs:201-208
`fs::remove_dir` already refuses non-empty folders. Drop the `read_dir` check and map `ErrorKind::DirectoryNotEmpty` to `FolderNotEmpty` in `friendly` (stable since Rust 1.83). Simpler, shorter, and no check-then-act gap. (Currently safe either way.)

### M5. Test gaps on the safety-critical paths
- No symlink tests at all (delete link-to-folder, copy/move onto dangling link, `show files` with a link). Add `#[cfg(unix)]` tests using `std::os::unix::fs::symlink`.
- `ask_yes` (runner.rs:72) reads the global stdin, so the confirm/decline path is untested. Make it `fn ask_yes(question: &str, input: &mut impl BufRead) -> bool` and test "yes", "y", "no", empty, EOF.
- Untested: `copy` of a folder -> `IsAFolder`; `rename`/`delete` of a missing item; `move` folder; `friendly()` mapping; `execute`/`run_line` outcomes (Failed vs Done vs Quit); `Display` for every `ActionError`.
- Parser: no test for `move a.txt into` (empty destination), joiner word in a name (`copy go to market.txt to x`), or uppercase `TO`.

## LOW

- L1. repl.rs:18-20, 30-33: stdout flush failure or stdin read error `return`s and `main` still exits SUCCESS (main.rs:26). Make `repl::run` return `ExitCode`/`bool` so an I/O failure exits non-zero.
- L2. runner.rs:76 (`Err(_) => false`): a stdin error while confirming is swallowed; at least `eprintln!` it. Declining a delete in one-shot mode exits 0 (runner.rs:29-30); consider a distinct non-zero or document it.
- L3. main.rs:22: `env::args()` panics on non-UTF-8 arguments. `env::args_os()` + `to_string_lossy()` avoids the panic. Also, one-shot args are re-joined with spaces (main.rs:30), so a quoted `"my  photos"` (two spaces) becomes `my photos`, and a shell glob (`delete *.txt`) fuses into one name; both fail safe (NotFound) but are surprising. Worth a line in help.
- L4. actions.rs:186 and 230: on case-insensitive filesystems (macOS default) `rename Notes.txt to notes.txt` is refused as AlreadyExists. Known limitation; maybe note it.
- L5. actions.rs:113: `entry.path().is_dir()` allocates a PathBuf and stats again; `entry.file_type()` is cheaper (decide how to treat links). The `(bool, String)` tuple with `is_file` and sort-by-false-first is clever; a tiny struct or `is_folder` (sorting `!is_folder`) reads better.
- L6. actions.rs:16: `ActionError` does not implement `std::error::Error`. One line (`impl std::error::Error for ActionError {}`) and a good learning point; enables `Box<dyn Error>` later. `Other(String)` drops the original `io::Error`; fine for v1.
- L7. actions.rs:12/91 and runner.rs:34: the clear-screen escape codes are printed even when stdout is a pipe/file. Consider skipping when not a terminal (`std::io::IsTerminal`, stable).
- L8. command.rs:52-58: `quote` only handles spaces; names with `"`, `$`, `'` produce a misleading "real command" hint. Display-only, so harmless; a comment saying so is enough.
- L9. suggest.rs: typing just `make` gives no hint (distance to "make folder" is 7). Add "make" handling in the parser (`["make"]` -> MissingName) for a friendlier message.
- L10. `println!` panics on a closed pipe (`noob show files | head -1`). Acceptable for a learning tool.
- L11. Nit: runner.rs:44 `command == Command::Bye` can be `matches!(command, Command::Bye)` (no comparison/clone semantics needed). Public items lack `///` docs in parser.rs/runner.rs/repl.rs (`pub fn parse`, `run_line`, `run`), though consistent with the heavy inline commenting elsewhere.

## Positives worth keeping
`create_new` in `make_file`; `symlink_metadata` in `delete`; `Drop`-based `TestFolder`; slice-pattern parser with names preserved vs lowercase keys; exhaustive `match` on `Command` (no wildcard in `real_command`); `?`/`map_err` used consistently.
