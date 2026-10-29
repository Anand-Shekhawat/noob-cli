# Noob CLI

A beginner-friendly command line. Instead of remembering `mkdir`, `cp` or `mv`,
you type plain English, and Noob shows you the real command it matches,
so you learn the real ones as you go.

```
noob> make folder photos
✓ Made folder "photos"
  (real command: mkdir -p photos)
```

## Try it

```bash
cargo run
```

That opens the `noob>` prompt. Type `help` to see everything, or `bye` to leave.

You can also run a single phrase:

```bash
cargo run -- make folder photos
```

To use `noob` from any folder, install it once:

```bash
cargo install --path .
```

## What it understands

| Type this | Real command |
|---|---|
| `where am i` | `pwd` |
| `show files` | `ls` |
| `make folder photos` (or `make dir`) | `mkdir -p photos` |
| `make file notes.txt` | `touch notes.txt` |
| `read notes.txt` | `cat notes.txt` |
| `copy a.txt to b.txt` | `cp a.txt b.txt` |
| `move a.txt into docs` | `mv a.txt docs` |
| `rename old.txt to new.txt` | `mv old.txt new.txt` |
| `delete notes.txt` | `rm notes.txt` |
| `clear` | `clear` |
| `help`, `bye` | |

Safety rules: Noob never overwrites an existing file, asks before deleting,
and only deletes empty folders.

## How the code is organised

| File | Job |
|---|---|
| `src/main.rs` | Starts the program: one phrase, or the `noob>` prompt |
| `src/repl.rs` | The `noob>` prompt loop |
| `src/runner.rs` | Runs one line: understand, confirm, do it, print |
| `src/parser.rs` | Turns your words into a `Command` |
| `src/command.rs` | The list of commands, and their real terminal versions |
| `src/actions.rs` | Does the work with files and folders |
| `src/suggest.rs` | The "Did you mean...?" typo helper |
| `src/help.rs` | The help text |

## Checks

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```
