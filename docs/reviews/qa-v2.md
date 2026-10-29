# QA report: Noob CLI v2 (feat/noob-v2)

Method: manual use of target/debug/noob in /private/tmp/claude-501/qa (HOME and NO_COLOR set, never answered "yes" to delete).
Testing stopped early (step limit). No tests/qa_edge_cases.rs written. `cargo test` was not run by QA.
Not covered: Tab-completer end-to-end, Ctrl+C, Trash behaviour.
Completer helpers were checked in a scratch copy only.

## MEDIUM
1. read/find on a FIFO hangs forever.
   Repro: `mkfifo pipe; noob read pipe` (also `noob find x in pipe`).
   Expected: refuse as not a regular file. Actual: blocks until killed.
2. Copying a folder that contains a FIFO hangs forever.
   Repro: `mkdir src; mkfifo src/fifo; noob copy src to dst2`.
3. Failed folder copy leaves a half-copied target and gives no warning.
   Repro: `src/nested/b.txt` with chmod 000; `noob copy src to dst`. It errors, but `dst` holds a.txt, z.txt and nested/. Running it again makes dst/src.
4. Symlinks are followed, against the promise.
   - `copy link.txt to copy1` makes a real file with the target's content.
   - `copy linkdir to x` copies the target folder's real contents.
   - `read link.txt` shows the target's content.
   - `add more to link.txt` appends into the file outside the folder. `write` does the same after its yes.
5. Text glued to a closing quote is silently joined with a space.
   Repro: `make folder "my photos"x` makes the folder "my photos x". It should error or keep one token.
   This also bites the completer. Its folder replacement is `"d/my dir/"`, with the quote closed, so you cannot keep typing inside the folder.

## LOW
6. Case-only rename fails on macOS.
   Repro: `rename a.txt to A.TXT` says "A.TXT already exists" (the case-insensitive disk counts it as taken).
7. `delete selfl`, where selfl is a symlink to the current folder, is refused. Same for links to home or to a parent. Only the link would be removed. check_delete canonicalizes the link.
8. `~` is not expanded.
   Repro: `go into ~` or `show files ~/x` gives "couldn't find ~". `go home` works.
9. Passing the whole phrase as one shell argument gives a nonsense hint.
   Repro: `noob "make folder"` says `I don't know "make folder". Did you mean "make folder"?`
10. Misleading real-command line: `write -n to f` shows `echo ./-n > f`. The ./ rule is applied to text, not just file names.
11. Wrong "not found" wording for bad paths.
    Repro: `make file nodir/x.txt`, `rename a to x/` and `write hi to x/` all say "couldn't find ..." about the wrong thing (the folder, or the trailing-slash name).
12. `read` prints a 5 MB single-line file in full. The cap is 200 lines but there is no character cap. NUL and escape characters are printed raw (`read bin.dat`, `find` matches).
13. `find text in .` silently skips non-UTF8 files and files over 5 MB, with no note. Hidden folders are skipped too.
14. `noob make folder " "` creates a folder named a single space.
15. Exit codes:
    - `noob help nosuch` returns 0.
    - Prompt mode piped from a script always returns 0, even when commands fail.
16. `find file in x` matches "find file" and searches for a file named "in x". Unquoted text containing "in" splits at the first "in": `find built in function in notes`.
17. Completer name matching is case-sensitive (`read NO` will not complete notes.txt on macOS). Tab after `find ` offers only `find file` and `find folder`.

## Checked and fine
- Delete refusals for ., .., /, ./, sub/.., home, parent, absolute parents, ../cwd.
- Move/rename into itself.
- Never-overwrite on copy, move, rename and make.
- Unicode names.
- Tabs and odd quotes: an unclosed quote is reported.
- Read caps at 200 lines and 10 MB.
- Write asks before replacing.
- Names starting with "-".
- Tree and details output.
