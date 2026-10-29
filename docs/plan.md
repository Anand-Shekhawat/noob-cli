# Noob CLI v2 plan

Approved 2026-10-29. Audience: beginners in general (public release later).

| Stage | What | Notes |
|---|---|---|
| 1 | One command table (`src/table.rs`) | Parser, typo help, help text and Tab completion read it |
| 2 | `"quoted names"` | So names can contain spaces and the word "to" |
| 3 | `go into` / `go back` / `go home` | Prompt mode only; prompt shows the folder |
| 4 | History + Tab completion | `rustyline`; own file-name completer that understands quotes |
| 5 | Colours | Off when not a terminal or when `NO_COLOR` is set |
| 6 | More commands | show all files / details / tree / size, find file, find text, write, add, open, copy folders, delete to Trash (`trash`) |
| 7 | Tests that run the real binary, README, help | |

Crates approved: `rustyline`, `trash`. Everything else uses the standard library.

Later (v3): teaching mode, custom phrases, shipping (Homebrew, CI, Windows testing).
