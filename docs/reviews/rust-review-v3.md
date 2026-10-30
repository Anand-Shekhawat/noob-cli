# Rust review: feat/v3-install (main...HEAD)

Verified on macOS: `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --check` clean,
`cargo test` 141 + 13 pass. Windows/Linux NOT compiled here (no target installed); cfg review is by reading.
No `unsafe`, no `unwrap` in production paths, no shell interpolation (args go to `Command::args`). Verdict: WARNING (no CRITICAL; two HIGH worth fixing).

## HIGH
1. src/tools/system.rs:96 `find_in` uses `file.is_file()`. On Windows, `winget.exe` lives in
   `%LOCALAPPDATA%\Microsoft\WindowsApps` as an app-execution-alias (reparse point). `metadata()` on these has
   historically failed (os error 1920), making `is_file()` false -> "winget isn't available" on a healthy PC.
   Unverified on 1.99; test on a real Windows box. Safer: accept `symlink_metadata().is_ok()` on Windows, or
   `file.exists() || fs::symlink_metadata(&file).is_ok()`. CI cannot catch it (tests use FakeSystem).
2. src/tools/package.rs:76-98 `winget_ids` assumptions: (a) `version_start - id_start` is unchecked usize
   subtraction (panic in debug / huge `take` in release if "Version" precedes " Id "); use `saturating_sub`.
   (b) Columns counted in `char`s but winget aligns by display width, so CJK/emoji names shift the Id column
   and yield garbage ids. (c) A footer line longer than `id_start` (e.g. "N results truncated...") becomes a bogus id.
   (d) Localised headers ("Id"/"Version" translated) return empty silently -> "couldn't find". Prefer
   `winget search ... --accept-source-agreements` plus a header check that errors visibly, and drop rows that
   contain spaces in the id.

## MEDIUM
3. src/tools/mod.rs:112 `after_homebrew(...)?` runs after a successful install; if appending to ~/.zprofile fails the
   whole command returns Err and the user thinks Homebrew failed. Report as a warning appended to the message.
4. src/tools/mod.rs:175-183 `installed_with` and package.rs:203-207 `exact_package` use `is_ok_and(worked)`:
   a broken brew, no network, or spawn failure looks like "not installed / not found". Effects: install proceeds
   or says "isn't installed with Homebrew, nothing to remove" (mod.rs:260) when brew itself failed. Return
   `Result<bool, ActionError>` or at least distinguish Err from `worked == false`.
5. src/tools/mod.rs:376-396 `is_installed` calls `exact_package(name, ...)` with an unvalidated name (no `valid_name`
   check, unlike `choose`): `is -x installed` passes `-x` as an option to `brew info`. Also `package::manager(..).ok()`
   hides why the manager part was skipped. Add the `valid_name` guard.
6. src/tools/mod.rs:426 `search_tools`: `package::manager(system)?` discards curated results when Homebrew/winget is
   missing; and `valid_name(text)` (427) silently skips the installer search for multi-word text like "video editor".
   Show curated hits plus a note instead.
7. src/real/unix.rs:59 passes `Os::Mac` even on Linux, so the "real command" for a cask tool shows `brew install --cask`
   on Linux (where install refuses). Use a real `Os::current()` (or pass Os into `for_shell`). Related duplication:
   the 7 tool variants are listed in 4 places (mod.rs:28, mod.rs:46 with `_ =>`, unix.rs:53, powershell.rs:81,
   actions/mod.rs:90). Wildcards at mod.rs:54/481 hide new variants; consider `Command::is_tool()` and
   `_ if command.is_tool()` once.
8. src/tools/mod.rs:477-480 `real_command` prints `brew search {text}` / `which {program}` unquoted; text with spaces or
   `$()` is wrong/dangerous when pasted. Reuse the shells' `quote`. Name in `Install(name)` display likewise.
9. src/tools/setup.rs:29 `read_to_string(..).is_ok_and(contains)`: non-UTF-8 or unreadable ~/.zprofile counts as
   "line missing" and a duplicate gets appended on each run. Distinguish NotFound from other errors.
   setup.rs:49-52 also assumes bash on Linux (zsh users get nothing useful) and maps Windows to .zprofile (dead branch).
10. Tests: no coverage for winget_ids edge cases above, `search`/`show_installed` errors (ask returns Err), installer
   `run` returning Err, `uninstall` of a Script tool, or the Linux Os path. src/tools/tests.rs:284 temp dir is not
   cleaned if an assert fails (use TestFolder-style Drop guard). `find_in` test (system.rs:116) doesn't cover Windows
   PATHEXT or non-executable files.

## LOW
11. src/actions/mod.rs:53-54 duplicated doc line "/// Runs one command." twice.
12. src/tools/system.rs:96-100 `find_in` doesn't check the executable bit on Unix and ignores PATHEXT on Windows;
   `possible_files` allocates a Vec per PATH folder (fine, but an iterator/`Option` is simpler).
13. src/tools/mod.rs:99-104 builds `message` then overwrites it for XcodeTools; write as `let mut message = if .. {..} else {..}`.
14. src/tools/mod.rs:157,193 hard-coded `/usr/bin/xcode-select` and `/bin/bash` (fine today, Mac/Linux only; add a comment
   or `cfg(unix)` so a future Windows Script entry fails loudly). `std::path::` used inline instead of imports.
15. src/tools/package.rs: `install_args` returns `Vec<String>` that every caller converts back to `Vec<&str>` (mod.rs:179,202);
   `words()` allocs for fixed strings. Readable, but `Vec<String>` -> `&[&str]` churn is avoidable. Public items
   (`Manager::name`, `Package` methods, `System`, `RealSystem`) lack `///` docs. `Manager` has no `Debug` derive.
16. src/tools/mod.rs:416 `text.to_lowercase()` recomputed inside the per-tool filter closure; hoist it.
17. src/real/unix.rs:104 `quote("")` returns an empty string (plain check passes vacuously); should be `''`.
18. src/actions/paths.rs:102 Windows `same_thing` returns false when `canonicalize` errs (e.g. target not yet existing, which
   is the normal case for rename) -> then `check_free` runs, which is correct; add a comment saying so.
19. src/repl.rs `NOOB_HISTORY=off` literal match is case-sensitive and a file named "off" can't be used; fine, document it.

## Positives
Trait boundary (System/FakeSystem with RefCell, `&self` methods) is clean and keeps tests off the real machine.
`Ask` alias is simple. File sizes fine: tools/mod.rs 483 lines, longest fn ~48 lines (install). Package names are
validated before reaching installers and passed as argv, not via a shell. cfg splits use `cfg!()` for logic and
`#[cfg]` only for items with platform APIs (unix::MetadataExt, symlink), which compiles on all three OSes by reading.
