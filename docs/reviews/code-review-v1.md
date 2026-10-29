# Code review v1 - Noob CLI
cargo test: 36 passed. Binary tried in /private/tmp/claude-501/ only. Safety design is good (no overwrite, confirm on delete, empty-folder-only).

## CRITICAL / HIGH
None.

## MEDIUM
1. src/actions.rs:180 and :189 - move/rename use friendly(error, from). If the DESTINATION folder is missing (`rename a.txt to nodir/b.txt`, `move a.txt to nodir/b.txt`) the user sees `I couldn't find "a.txt"` although a.txt exists (verified). Use the destination name / a "destination folder doesn't exist" message. Test missing.
2. src/command.rs:42-50 quote() - only quotes names containing a space. Names starting with "-" (`delete -n`) print `rm -n`, which real rm treats as an option (verified); names with ' " $ * & ; ( ) print commands that do something different if pasted. Teaching tool: should add `--` for leading "-" and quote with single quotes (escape '). Only space and plain names are tested.
3. src/command.rs:31-33 vs src/actions.rs:177-191 - printed `mv` would overwrite an existing target, Noob refuses. Printed `mkdir -p` succeeds silently on existing folder, Noob errors. Fine for safety, but the lesson "real command" differs from what happened; add one line (or help note) that Noob is stricter than the real command. Same for `cp` into a folder: shows `cp a.txt docs` (correct in real shell) while message says docs/a.txt - fine, just worth a note.
4. src/actions.rs:180 - `move docs into docs` gives "Something went wrong: Invalid argument (os error 22)" (verified). Add friendly message for moving a folder into itself; cross-drive moves fail similarly (rename can't cross filesystems).
5. src/runner.rs:26 (known issue) - confirm precedes existence check. Also `delete .` / `delete ..` / `delete ~`-style paths just ask the generic question; consider refusing "." and ".." and naming what will be deleted (file vs folder).

## LOW
6. src/parser.rs:70-73 - first "to"/"into" always splits, so `copy go to sleep to docs` or a file literally named "to" cannot work and no hint is given. The printed command shows what was understood, but a one-line help note ("names can't contain the word to") would help. Case: joiner is case-insensitive, names keep case - correct.
7. src/parser.rs:33 - `make` alone gives "Did you mean move?" (verified) because the suggestion compares only 1 word to 'move'. Better: "make what? try make folder photos". `make folder x` accepted for names like "a/b/c" silently creates nested folders - intended, matches mkdir -p.
8. src/actions.rs:156-165 - `read` prints whole file unbounded; a large text file or one with terminal escape codes floods/garbles the screen. Cap size or note it.
9. src/runner.rs:72-78 - in one-shot mode with piped/closed stdin delete is silently cancelled (fine, safe), but text "Type yes to continue" ends with ":" and no newline-prompt; trivial.
10. Test gaps: no tests for runner.rs (explain messages, confirm flow), repl.rs, main.rs exit codes; no test for names with "to" inside (e.g. `copy photo.txt to tomorrow`, "tools" - works via whole-word match, but untested), no test for case-insensitive joiner (`COPY a TO b`), unicode names, move/rename to a missing folder, move into itself, or delete of a symlink.
