# Noob CLI

A command line for beginners. Instead of remembering `mkdir`, `cp -R` or `grep -rni`,
you type plain English. Noob shows you the real command it matches, so you learn the
real ones as you go.

Noob comes two ways:

- **The Noob app**: a real terminal (your own shell, with tabs) and a **Noob bar**
  along the bottom. Type `make folder photos` in the bar, and Noob writes
  `mkdir -p photos` into the terminal for you. You press Enter to run it.
- **The `noob` command**, for the terminal you already use:

```
noob photos> copy cat.jpg to backup
✓ Copied "cat.jpg" to "backup/cat.jpg"
  (real command: cp cat.jpg backup)
```

## Install

**Mac** (and Linux), with [Homebrew](https://brew.sh):

```bash
brew install anand-shekhawat/tap/noob
```

**Windows**, in PowerShell:

```powershell
irm https://raw.githubusercontent.com/Anand-Shekhawat/noob-cli/main/scripts/install.ps1 | iex
```

It installs for just you (no administrator needed), checks the download's
fingerprint (its SHA-256 checksum), and adds Noob to your PATH. To install one exact
version: `& ([scriptblock]::Create((irm https://raw.githubusercontent.com/Anand-Shekhawat/noob-cli/main/scripts/install.ps1))) -Version v1.0.0`. Noob isn't code-signed yet, so Windows may
show a SmartScreen warning the first time: choose "More info", then "Run anyway".

**From the source code**, if you have Rust: `cargo install --path .`

Then type `noob` to open the `noob>` prompt. Type `help` to see everything, or `bye` to leave.

- **Tab** finishes what you're typing: commands first, then file names.
- **Up arrow** brings back earlier commands, even after you close Noob.
- **Ctrl+C** clears the line. **Ctrl+D** leaves.

You can also run one phrase straight from your normal terminal: `noob make folder photos`.

## The Noob app

The app is a real terminal: it runs your usual shell (zsh on a Mac, PowerShell on
Windows), with colours, full-screen programs like `nano` and `top`, and copy and paste.
Along the bottom is the **Noob bar**, where you can ask in plain English.

- **Mac**: `brew install --cask anand-shekhawat/tap/noob-app`, or download
  `Noob-…-apple-darwin.zip` from the [latest release](https://github.com/Anand-Shekhawat/noob-cli/releases/latest)
  (`aarch64` for Macs with Apple chips, `x86_64` for Intel) and drag `Noob.app` into Applications.
- **Windows**: the PowerShell installer above also installs the app and adds
  **Noob** to your Start menu.

Noob isn't signed by Apple yet (that costs money each year), so the first time you
open it, macOS may say it can't be opened. Go to **System Settings → Privacy &
Security** and click **Open Anyway** next to Noob. You only need to do this once.

### The Noob bar

Press **⌘K** (Ctrl+K on Windows, Ctrl+Shift+K on Linux) or click the bar, and type
what you want, like `make folder photos` or `delete old.txt`.

- Noob writes the real command into the terminal and says in one line what it does.
  **Nothing runs until you press Enter** in the terminal yourself.
- Risky things (deleting, replacing a file's text, installing) ask first, with Yes and No.
- Deleting always goes to the Trash (`trash` on a Mac), never `rm`.
- Names mean the files in the folder your terminal is in, and **Tab** finishes them.
- If a program like `nano` is open, the bar waits and keeps what you typed.
- `help` lists everything Noob understands.

### Tabs

**⌘T** opens a new tab in the folder you were in, **⌘W** closes it, **⌘1–9** switch,
and **⌘⇧[ / ⌘⇧]** go left and right (on Windows and Linux: **Ctrl+Shift** with the
same keys). Tabs you aren't looking at keep running, and get a dot when something new
appears. Closing a tab where something is still running asks first.

### Settings

**⌘+ / ⌘− / ⌘0** make the letters bigger, smaller or normal (Ctrl on Windows and
Linux). **⌘,** opens Settings, with four colour choices: follow my computer, dark,
light, and high contrast. Noob remembers them in a small text file you can read:
`~/Library/Application Support/Noob/settings.txt` on a Mac,
`%APPDATA%\Noob\settings.txt` on Windows, `~/.config/noob/settings.txt` on Linux.

### Known limits

- On Windows, the Noob bar can't yet see which folder PowerShell is in, so its
  checks (like "does this file exist?") look in your home folder. Commands it
  types still run in the right place.
- On Windows, letters typed with AltGr (like `@` on a German keyboard) don't reach
  the terminal yet.
- Nobody has tried the app on a real Windows PC yet (it's built and tested on
  Windows automatically). If you do, please tell us how it went.

## Installing tools and apps

Noob installs things for you with Homebrew on a Mac and winget on Windows
(it comes with Windows 10 and 11). On a new Mac, start with `install homebrew`.

| Type this | What happens |
|---|---|
| `show tools` | the 15 tools and apps Noob knows by name |
| `install git` | installs Git (also: `install vscode`, `install python`, `install chrome`...) |
| `install ripgrep` | anything else, by its exact Homebrew or winget name |
| `search tools video` | look for more |
| `is git installed` | yes or no, and where it is |
| `show installed` | everything installed with Homebrew or winget |
| `update everything` | the newest version of everything |
| `uninstall vlc` | removes it |

Noob always says what it will install and where it comes from, then asks first.
Installers run on screen, so if one asks for your password, you're typing it to
macOS or Windows directly: Noob never sees it.

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
| `clear`, `help`, `help copy`, `version`, `bye` | |

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

## How downloads are checked

- Tools come only from Homebrew, winget, or the tool's own official installer
  (brew.sh, rustup.rs). Noob refuses names that would add someone else's Homebrew
  source (names with `/`).
- Noob's own downloads come with a SHA-256 fingerprint. The Windows installer checks
  it, which catches damaged downloads; the Homebrew formula keeps its own copy of the
  fingerprints in a separate repository, which also catches a swapped download.
- Noob isn't code-signed (that costs money each year), so macOS and Windows may warn
  the first time you open a downloaded copy.

## Privacy

The commands you type in the `noob>` prompt are saved to `~/.noob_history` so the
up arrow works next time, just like `~/.zsh_history`. Delete that file to clear it.
To keep one line out of the history (say, a password you `write`), start it with a space.
To keep the history somewhere else, set `NOOB_HISTORY` to a file path, or to `off` for none.

Colours switch off automatically when the output isn't a terminal, or when the
`NO_COLOR` environment variable is set.

## How the code is organised

| File | Job |
|---|---|
| `src/lib.rs` | Noob's engine as a library, shared by the terminal `noob` and the app |
| `src/engine.rs` | Runs one line: understand, check, ask, do it, and describe what happened |
| `src/main.rs` | The terminal `noob`: one phrase, or the `noob>` prompt |
| `src/app/` | The Noob app (`--features app`): `main.rs` starts it, `view.rs` lays out the window |
| `src/app/terminal/` | Drawing the terminal: keys to bytes, colours, selection, scrolling |
| `src/app/bar.rs` | The Noob bar under the terminal |
| `src/app/tabs/` | Tabs: the tab list, shortcuts and tab names |
| `src/app/settings/` | Letter size and colours, and the settings file |
| `src/term/` | Noob's own terminal engine: `pty` runs your shell, `parser` reads its escape codes, `grid` keeps the screen, `folder` finds the shell's folder |
| `src/bar.rs` | Decides what the Noob bar does with a sentence (type a command, ask first, or answer) |
| `src/chat.rs`, `src/terminal.rs` | The old chat window's logic; no longer used by the app (to be removed) |
| `src/complete.rs` | Tab completion, used by both |
| `src/table.rs` | **The command table**: every phrase, example and help line, written once |
| `src/parser.rs` | Turns your words (and "quoted names") into a `Command` |
| `src/command.rs` | The list of commands, and their real terminal versions |
| `src/runner.rs` | The terminal's side of running a line: asks from the keyboard, prints in colour |
| `src/repl.rs` | The `noob>` prompt, history and keys (uses `rustyline`) |
| `src/completer.rs` | Connects Tab completion to the terminal's keyboard handling |
| `src/suggest.rs` | "Did you mean...?" |
| `src/help.rs` | `help` and `help <command>`, built from the table |
| `src/style.rs` | Colours |
| `LICENSE` | MIT: anyone may use, change and share Noob, keeping the copyright notice |
| `src/actions/` | The real work, one file per kind of job (files, listing, moving, finding...) |
| `src/tools/` | Installing tools: the curated catalog, Homebrew and winget |
| `src/real/` | The real commands Noob teaches: Mac/Linux and PowerShell |
| `tests/cli.rs` | Tests that run the real `noob` program |

### Adding a command

1. Add a variant to `Command` in `src/command.rs`.
2. Add an entry to `COMMANDS` in `src/table.rs` (phrases, example, summary).
3. Run `cargo build`. The compiler points at every place that needs the new
   command: its real command, and its action in `src/actions/mod.rs`.

The table tests check that your example really runs your command.

## Making a release

1. Put the new version in `Cargo.toml`, commit, and merge to `main`.
2. Tag it and push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
   GitHub builds Noob for Mac (Apple chip and Intel), Windows and Linux, attaches
   the downloads to a Release, and tries the Windows installer on a fresh PC.
3. Update the Homebrew tap:
   `scripts/homebrew-formula.sh vX.Y.Z > ../homebrew-tap/Formula/noob.rb` and
   `scripts/homebrew-cask.sh vX.Y.Z > ../homebrew-tap/Casks/noob-app.rb`, then commit
   and push the tap.

## Checks

```bash
cargo run --features app --bin noob-app   # try Noob's window
cargo test --features app
cargo clippy --all-targets --features app -- -D warnings
cargo fmt --check
```
