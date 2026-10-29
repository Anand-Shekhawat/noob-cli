# Noob CLI

A command line for beginners. Instead of remembering `mkdir`, `cp -R` or `grep -rni`,
you type plain English. Noob does it safely, then shows you the real command it
matches, so you learn the real ones as you go.

```
noob photos> copy cat.jpg to backup
✓ Copied "cat.jpg" to "backup/cat.jpg"
  (real command: cp cat.jpg backup)
```

## Try it

```bash
cargo run
```

That opens the `noob>` prompt. Type `help` to see everything, or `bye` to leave.

- **Tab** finishes what you're typing: commands first, then file names.
- **Up arrow** brings back earlier commands, even after you close Noob.
- **Ctrl+C** clears the line. **Ctrl+D** leaves.

You can also run one phrase straight from your normal terminal:

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
| **Getting around** (inside the `noob>` prompt) | |
| `where am i` | `pwd` |
| `go into photos` (or `go to`) | `cd photos` |
| `go back` (or `go up`) | `cd ..` |
| `go home` | `cd ~` |
| **Looking at things** | |
| `show files` (or `show files photos`) | `ls` |
| `show all files` (includes hidden ones) | `ls -a` |
| `show details` (sizes, when things changed) | `ls -l` |
| `show tree` | `ls -R` |
| `show size photos` (or `how big is`) | `du -sh photos` |
| `read notes.txt` | `cat notes.txt` |
| **Making and changing** | |
| `make folder photos` (or `make dir`, `new folder`) | `mkdir -p photos` |
| `make file notes.txt` | `touch notes.txt` |
| `write "hello" to notes.txt` | `echo hello > notes.txt` |
| `add "buy milk" to list.txt` | `echo 'buy milk' >> list.txt` |
| `open notes.txt` | `open notes.txt` |
| **Copying, moving, deleting** | |
| `copy a.txt to b.txt` (files or whole folders) | `cp a.txt b.txt` / `cp -R` |
| `move a.txt into docs` | `mv a.txt docs` |
| `rename old.txt to new.txt` | `mv old.txt new.txt` |
| `delete notes.txt` (or `remove`) | `rm notes.txt` |
| **Finding** | |
| `find file cat` | `find . -iname '*cat*'` |
| `find "hello" in notes` | `grep -rni hello notes` |
| **Other** | |
| `clear`, `help`, `help copy`, `bye` | |

Names with spaces work with or without quotes: `make folder my photos`.
The **last** joining word counts, so `write go to the shop to list.txt` works.
Use quotes when a name after the joining word contains it: `copy a.txt to "to do"`.
`~` means your home folder, like in a real terminal: `go into ~/Desktop`.

## Safety rules

- **Delete sends things to the Trash**, so you can always get them back. Noob asks
  first and tells you how much is inside a folder.
- Noob won't delete the folder you're in, any folder holding it, your home folder,
  or the whole disk.
- Noob **never overwrites**: copy, move, rename and make refuse if the name is taken.
  `write` asks before replacing text that's already in a file.
- Shortcuts (symlinks) are never followed when copying, searching or deleting,
  and Noob won't write through them.
- Devices and pipes (special files that could make a program wait forever) are
  left alone.
- The real commands Noob shows are less careful: `cp` and `mv` replace files
  without asking, and `rm` skips the Trash. Noob reminds you about `rm`.

## Privacy

The commands you type in the `noob>` prompt are saved to `~/.noob_history` so the
up arrow works next time, just like `~/.zsh_history`. Delete that file to clear it.
To keep one line out of the history (say, a password you `write`), start it with a space.

Colours switch off automatically when the output isn't a terminal, or when the
`NO_COLOR` environment variable is set.

## How the code is organised

| File | Job |
|---|---|
| `src/main.rs` | Starts the program: one phrase, or the `noob>` prompt |
| `src/table.rs` | **The command table**: every phrase, example and help line, written once |
| `src/parser.rs` | Turns your words (and "quoted names") into a `Command` |
| `src/command.rs` | The list of commands, and their real terminal versions |
| `src/runner.rs` | Runs one line: understand, check, ask, do it, print |
| `src/repl.rs` | The `noob>` prompt, history and keys (uses `rustyline`) |
| `src/completer.rs` | Tab completion |
| `src/suggest.rs` | "Did you mean...?" |
| `src/help.rs` | `help` and `help <command>`, built from the table |
| `src/style.rs` | Colours |
| `src/actions/` | The real work, one file per kind of job (files, listing, moving, finding...) |
| `tests/cli.rs` | Tests that run the real `noob` program |

### Adding a command

1. Add a variant to `Command` in `src/command.rs`.
2. Add an entry to `COMMANDS` in `src/table.rs` (phrases, example, summary).
3. Run `cargo build`. The compiler points at every place that needs the new
   command: its real command, and its action in `src/actions/mod.rs`.

The table tests check that your example really runs your command.

## Checks

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```
