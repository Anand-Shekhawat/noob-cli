# Rust review: feat/noob-v2 (vs feat/noob-v1)

Checks run: `cargo clippy --all-targets -- -D warnings` clean, `cargo fmt --check` clean,
`cargo test` 91 unit + 11 integration pass. cargo-audit/deny not installed (not run).
No file over 400 lines; no function over ~50 lines. No `unsafe`, no non-test `unwrap`/`expect`.

Verdict: APPROVE (no CRITICAL, no HIGH). Several MEDIUM/LOW items below.

## CRITICAL / HIGH
None.

Verified OK (asked about):
- completer.rs:27 `&line[..cursor]`: rustyline passes a char-boundary byte offset. completer.rs:67-71
  slice at an ASCII `"` / space / `/` found by `rfind` (+1), always a boundary. No panic on non-ASCII.
- parser.rs tokenizer works on `chars()`, no byte indexing. finding.rs `shorten` is char-based.
- No unit test calls `set_current_dir`; only failing paths of `go_into` are unit-tested, successes in tests/cli.rs.
  `TestFolder` names are unique today (checked for duplicates).
- `#[cfg(test)] send_to_trash` is only compiled into unit tests; tests/cli.rs runs the real binary
  (never answers "yes"), so the real Trash is not touched today.

## MEDIUM
1. src/actions/files.rs:51 and finding.rs:~98 (`find_text` single file): `read_to_string` after a
   size check on `metadata.len()`. Special files report len 0, so `read /dev/zero` reads without
   limit (memory blow-up) and a FIFO blocks forever. Fix: check `metadata.is_file()` and/or read
   via `File::open(..).take(MAX_READ_BYTES + 1)`.
2. src/style.rs:15: colour is decided from stdout only, but errors/warnings go to stderr
   (runner.rs:41,87). `noob ... 2>err.log` with a terminal stdout writes escape codes into the file
   (and the reverse case: no colour on a terminal stderr). Use per-stream `enabled(stream)` or
   check `stderr().is_terminal()` for the stderr helpers. Also `TERM=dumb` is not honoured.
3. src/command.rs:~135 `opener()` uses `explorer` on Windows; explorer.exe commonly exits with code 1
   even on success, so files.rs:147 reports a false "couldn't open". Treat Windows specially
   (`cmd /C start "" path`) or skip the status check there. (Unverified: no Windows machine.)
4. src/actions/moving.rs:146-181: the test stand-in for Trash deletes permanently and the real path
   (`trash` crate error mapping, macOS `NsFileManager` method) has zero test coverage; safety of the
   integration tests rests on a comment ("no test answers yes"). Teaching-friendly alternative:
   pass a `fn(&Path) -> Result<..>` into `delete`, or a tiny `Trasher` trait, so tests inject a fake
   and no `cfg(test)` fork exists.

## LOW
- completer.rs:32: first word with no phrase match (e.g. `xyz` + Tab) falls through to file-name
  completion with start=0 and would replace the whole line with a file name. Only fall back to
  `complete_name` when `typed` contains a space (or a phrase already matched).
- completer.rs:63-99: `complete_name` (the filesystem part) has no tests; add a TestFolder test, and
  one with a non-ASCII name (`caf\u{e9}`) to lock in the boundary reasoning. `~/` is not expanded;
  `rfind(' ')` ignores tabs.
- parser.rs:48-52: `a"b c"d` becomes one word containing quotes, `"a""b"` becomes two tokens ("a b").
  Fine for a beginner tool; note it in a comment/test.
- runner.rs:30: when words come from the shell, quotes are already gone, so `noob copy to docs` /
  a file literally named `to` cannot be expressed. Known limitation; document in README/help.
- runner.rs:36: `in_prompt: bool` flag; a two-variant enum (`Mode::Prompt/OneShot`) reads better for
  learners.
- moving.rs:105-108: `.ok()` on `current_dir`/`canonicalize` skips the "precious" check silently
  if either fails (e.g. cwd deleted, permission). Add the "intentional" comment or refuse the delete.
  Also `canonicalize` follows symlinks, so `delete link-to-cwd` is refused although only the link
  would go to the Trash (safe direction, but surprising).
- moving.rs:16,52,35-57: `copy` of a symlink-to-file/dir at top level follows the link, while inside
  folders links are copied as links (inconsistent; comment at :59 says "never the thing it points to").
  Non-unix `copy_shortcut` (:66) follows links, contradicting its doc. FIFOs/sockets inside a folder go
  to `fs::copy` and can hang. A failed `copy_folder` leaves a half-copied target with no mention.
- paths.rs:37 + moving.rs:94 / 16 / 52: check-then-act races (`check_free` then `fs::rename` /
  `fs::copy` silently overwrites on Unix). Acceptable for a local single-user tool; a one-line
  comment would teach the idea. `OpenOptions::create_new` for the copy target would close it.
- paths.rs:57-66 `is_inside` canonicalizes `source`, so moving a *symlink* into the folder it points to
  is wrongly "into itself".
- errors.rs:76: fallback `Other(error.to_string())` drops the file name; `InvalidData -> NotText` (:70)
  is applied to every operation (a non-read InvalidData would say "isn't a text file").
  `IsADirectory`, `ReadOnlyFilesystem`, `StorageFull` are unmapped.
- Silent `.ok()` / `unwrap_or_default` without the "intentional" comment: repl.rs:109,
  listing.rs:111,122-132 (shows "?" / empty), moving.rs:105-106, help.rs:45, paths.rs:75.
- listing.rs:31-33: one unreadable entry makes the whole `show files` fail, while `walk` tolerates
  and counts them. listing.rs:~131 `{:<width$}` pads by chars, so wide (CJK/emoji) names misalign.
  `show details` item counts include hidden files that the list hides. `path.is_dir()` per entry is a
  stat per file (`entry.file_type()` is cheaper; fine at this size).
- listing.rs:~176 `lines.len() > TREE_LINES` allows TREE_LINES+2 lines; cosmetic.
- finding.rs: `find text` in a folder prints results in directory order (not sorted, platform
  dependent) while `find file` sorts; `find file` stops at 50 in traversal order and then sorts, so
  which 50 you see is arbitrary.
- moving.rs:129 wording "and the N things inside it" counts only direct children.
- table.rs:74-87 `one`/`two` use `unwrap_or_default`, hiding a parser/table mismatch as an empty
  name. Safe today (empty delete is refused, empty open is NotFound) but a `debug_assert!` would catch
  a future bug.
- main.rs:26-29: non-UTF-8 argument names are lossy-converted, so such files can never be addressed
  (they fail with "couldn't find", not wrong-target). Comment says "copes"; say what it costs.
- Test gaps: runner `run`/`ask_yes` EOF/error paths (ask_yes takes `impl BufRead` for exactly this,
  only the yes/no parsing is tested), `check_delete` precious cases (`is_precious` is pure and
  trivially unit-testable), `Completer::complete`, `walk` unreadable counting, non-ASCII names in
  tokenizer/`real_command`/`quote`.
