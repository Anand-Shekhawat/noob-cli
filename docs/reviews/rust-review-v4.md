# Rust review: feat/app (v4)
Clippy (--all-targets --features app, -D warnings) and tests (--features app): clean, all pass.
Verdict: APPROVE with warnings (no CRITICAL/HIGH).

## CRITICAL / HIGH
None. Byte/char handling is correct: complete_name's `start` comes from rfind on ASCII ' ' / '"' (always a char boundary), with_choice uses `str::get(..start)` (no panic), cursor uses chars().count() (egui CCursor is char-based).

## MEDIUM
- src/app/view.rs:162 and 195-198: Tab completes the WHOLE `typed` (the rustyline adapter uses `line[..cursor]`). With the cursor mid-line, text after the cursor is silently discarded by with_choice. Either read the cursor from TextEdit state and complete `typed[..cursor]`, or document "Tab acts at end of line".
- src/app/view.rs:196: `.unwrap_or(&self.typed)` silently appends the choice to the full text if `start` is stale/invalid. Stale case is real: set_typed (up/down, lines 150-155) does not clear `self.choices`, so old Pick-one buttons remain with an old choices_start. Clear choices inside set_typed.
- src/chat.rs:158-165 (run): commands run synchronously on the UI thread, so copy/move/delete of big folders freezes the window. Acceptable for v1; note it or add a "working..." line.
- src/terminal.rs:102-111 (Linux): `x-terminal-emulator -e` behaves differently per terminal (gnome-terminal needs `--`); CI cannot test it. Child from `.spawn()` is dropped, never waited (zombie until app exit, Unix). Windows/Linux `open` bodies are untested.
- src/lib.rs:6-18: every module is `pub`. The window needs only chat, complete, engine, style, terminal; main.rs needs terminal. actions, command, help, parser, real, suggest, table, tools can be `pub(crate)` (check `engine::Line`/`Outcome`/`Chat::Entry` do not expose their types; Command is exposed via engine::Next, so keep `command` pub or make Next opaque). Smaller API = fewer accidental promises.

## LOW
- src/terminal.rs:36-46 (macOS): if `open` fails, the .command file is left in temp (only the script deletes itself). Remove it on the Err path.
- src/terminal.rs:13, chat.rs:31,40,51,62, complete.rs:13-30, style.rs:15: new pub items lack `///` (TerminalWindow, Handoff::run_in_terminal, Chat::new/entries, complete_phrase, turn_off).
- src/chat.rs:154: `Chat::complete(&self, ..)` ignores self; a thin wrapper. View could call complete::complete directly.
- src/chat.rs:79-97: folder()/folder_name() call current_dir() every repaint (a syscall per frame); cache after each send. view.rs:79 allocates the question String every frame.
- src/app/view.rs:115,203: Id::unique(INPUT) is rebuilt in two places; use one `const`/helper so they cannot drift.
- src/app/view.rs:34: `#[allow(unused_mut)]` has a reason comment (fine); could instead put the demo in a separate #[cfg] fn.
- src/app/main.rs:3,13: with windows_subsystem = "windows" an eframe::Result error is invisible on Windows; consider a message box-free fallback (write to a log file) later.
- src/terminal.rs:70: `display().to_string()` is lossy for non-UTF-8 paths (script would quote a wrong path). Rare.
- Test gaps: no tests for view.rs with_choice (move the string logic into chat/complete as a pure fn and test it, incl. multibyte "é" and quotes), complete() with quoted/multibyte input, style::turn_off, Chat MAX_ENTRIES trimming (check), history up/down edges.

## OK / verified
- cfg: macOS-only fns gated with `#[cfg(any(test, target_os = "macos"))]` compile on all three OSes; `Path` import used on all. Windows `open` and Linux `open` are mutually exclusive with macOS (unix-not-macos). 
- egui: consume_key before TextEdit, lock_focus for Tab, Enter via lost_focus + key_pressed, refocus logic: all correct for 0.37. Y/N/Escape consumed only while a question is showing.
- Ignored results (`let _ =` set_current_dir, read_line) are commented and harmless.
