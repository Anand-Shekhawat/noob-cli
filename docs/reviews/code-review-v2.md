# Code review: feat/noob-v2 vs feat/noob-v1

`cargo test`: 91 unit + 11 integration pass. Scenarios run in /private/tmp/claude-501/rv2..rv4 with a fake HOME. Nothing was answered "yes" to a delete, and `open` was not run. No source files were changed.

## HIGH

1. **`write`/`add` split on the FIRST " to "** (src/parser.rs:138-146, `Shape::Two`).
   - `write go to the shop to a.txt` wrote "go" into a new file named `the shop to a.txt`.
   - The text goes to the wrong file with no warning. The "real command" line shows it only afterwards.
   - If a file with that odd name already exists, it is the one that gets replaced (after the replace prompt).
   - Fix: for write/add, split on the LAST joiner, or ask the user to quote the text. At minimum, show the target before writing.

## MEDIUM

2. **`copy_folder` leaves a half-copied folder and doesn't say so** (src/actions/moving.rs:35-61).
   - With an unreadable `d/b`, `copy d to d2` printed only "won't let me touch d/b".
   - `d2` was left behind with part of the contents, so a retry says "d2 already exists... delete it first".
   - Fix: report "copied N of M, partial copy left in d2". Or clean up, or keep going and list the files that were skipped.
3. **`~` is never expanded** (src/actions/paths.rs, src/actions/moving.rs).
   - `copy a.txt to ~` created a file literally named `~`, and `show files ~` and `delete ~` say "couldn't find". Beginners will type `~`.
   - A file literally named `~` is a classic trap if someone later removes it in a real terminal without quotes.
   - Fix: expand a leading `~/` in the parser, or reject `~` with a hint to use `go home`.
4. **The printed "real command" is wrong or more dangerous than what Noob did** (src/command.rs:79-110, 141).
   - `quote()` turns a leading "-" into "./-". That is right for file names but wrong for `echo` text and `grep` patterns: `find "-n" in d` prints `grep -rni ./-n d`, which searches for the wrong thing. `write "-n hi"` has the same problem.
   - `mv`/`cp`/`echo >` silently overwrite, while Noob refuses to. The learner who copies the command can destroy data. Add `-n` or a note, as was done for `rm`.
   - `find`/`grep` also include hidden files, which Noob skips.
5. **`write` follows symlinks and treats a dangling one as free** (src/actions/files.rs:73-77, 103-111).
   - `write hi to lnk` (lnk -> real.txt) overwrote real.txt. The prompt and message only mention "lnk".
   - `write oops to dang` (dangling link) created the link's target `nothing`. copy/rename deliberately refuse this (the `taken()` rule), so write/add are inconsistent.
   - Fix: mention "(a shortcut to X)" in the prompt, or refuse dangling links.
6. **History keeps everything typed in plain text** (src/repl.rs:83).
   - The file is correctly mode 0600, but `write my password is hunter2 to n.txt` is stored forever in ~/.noob_history.
   - Fix: don't record write/add lines. Or tell the user in the README and the welcome text, and offer a way to turn history off.
7. **Thin tests on the riskiest paths.**
   - There is no test for `copy_folder` failing partway, copying symlinks, or write through a symlink.
   - `check_delete` is only tested for ".", ".." and "/". The home-folder guard is only tested via `is_precious` with fake paths.
   - `open` has no argument test. The `-name` case is untested, and so is the `completer` quote handling.

## LOW

8. **`check_delete` blocks symlinks wrongly** (src/actions/moving.rs:98-112).
   - `delete up` (up -> ..) said "that's the folder you're in, one above it". The link is harmless to remove.
   - It canonicalizes the link's target. Use the link itself when `symlink_metadata` says it's a symlink.
9. **Error texts are stale** (src/actions/errors.rs:35, 53).
   - `CantDelete` doesn't mention the home folder, which `check_delete` also guards.
   - `FolderNotEmpty` still says "Noob only deletes empty folders", but v2 deletes full folders to the Trash.
10. **Terminal escape codes in files run when displayed** (`read`, `find "text" in`, and `show files` file names).
    - Raw file content and names are printed. A hostile file can contain escape sequences. Strip control characters other than newline and tab.
11. **`find "text" in folder` silently skips non-UTF-8 files** (src/actions/finding.rs:~70) and still says "couldn't find". Mention how many files were skipped.
12. **TOCTOU between `check_free` and `fs::copy`** (src/actions/moving.rs:14-20) could overwrite a file created in between. Practically negligible. `create_new` would close it.

## Checked and fine

- `make file` uses `create_new`.
- The history file is 0600.
- Delete asks first, and EOF means "no".
- Delete goes to the Trash, and the test stand-in is `cfg(test)` only.
- The integration tests use a fake HOME and never answer yes.
- Symlinks are not followed by `walk` or `copy_folder`.
- `delete_question` uses `symlink_metadata`.
- `open` guards names that start with "-".
