# Code review: feat/v3-install (tools, install flows, catalog, README)

Verified: `cargo test` passes (141 unit + 13 integration). Ran read-only commands in a scratch folder.
No CRITICAL issues. Security and idioms are covered by the other reviewers
(one pointer: `install user/tap/name` is allowed by `valid_name` and `brew info` auto-taps; see security review).

## HIGH
None.

## MEDIUM
1. src/tools/mod.rs:155-172 (+ catalog.rs Cask entries): a Mac app already in /Applications but not installed by Homebrew
   (very common for Chrome, Firefox, Zoom, VLC, VS Code) is reported "not installed" by `is chrome installed`
   and `install chrome` runs `brew install --cask`, which refuses ("an App already exists"). Beginner sees a generic
   "installer stopped with a problem". Fix: for Cask tools also check /Applications/<App>.app (add an `app` name to
   the catalog) and say "already installed (not by Homebrew)".
2. src/tools/mod.rs:443-476 real_command for `is X installed`: uses the raw name when the tool has no `programs`,
   so output was `which chrome` and `which build tools` (verified). Pasting these is wrong/misleading. Return None
   (or `brew list --cask google-chrome` / `xcode-select -p`) for tools without programs.
3. src/tools/mod.rs:376-410 `is git/python installed` on a Mac without Command Line Tools finds /usr/bin/git and
   /usr/bin/python3 (Apple stubs that pop up the CLT dialog) and says "Yes". On Windows the Store alias
   python.exe in WindowsApps does the same. Ignore /usr/bin stubs unless `xcode-select -p` works, and ignore WindowsApps paths.
4. src/tools/mod.rs:384-395 and system.rs:78 / package.rs:215,241: quiet winget calls (`list`, `show`, `search`) lack
   `--accept-source-agreements`. On a fresh Windows PC winget asks to accept source terms; with stdin = null the call
   fails, so installs say "couldn't find", uninstall says "nothing to remove", search shows nothing. Add the flag to
   every `ask` call for winget (and `--accept-package-agreements` is fine to leave since install runs on screen).
5. src/tools/mod.rs:92-114 + catalog.rs:73: Homebrew script is `bash -c "$(curl -fsSL ...)"`. If curl fails (offline),
   the substitution is empty, bash exits 0, and Noob prints "Installed Homebrew." (then "But I can't find Homebrew now").
   Reply is Done, not an error. Fix: after a Script install, if the tool is Homebrew and `homebrew_location` is None,
   return an Explained error instead of "Installed".
6. src/tools/mod.rs:412-440 `search tools` and 338 `show installed` fail with "Homebrew isn't installed" before showing
   the curated list that needs no Homebrew. Show curated results first, then the Homebrew note.

## LOW
- README.md: says `show tools` lists "the 16 tools"; the catalog has 15 (verified).
- src/tools/mod.rs:427 `search tools video player` (two words) gives "Nothing matches"; one word works. Search by
  words individually or explain.
- src/tools/package.rs:260-298 winget_ids depends on English headers " Id "/"Version" and char-count columns
  (localized Windows or wide characters give empty results). show_installed (mod.rs:360) prints raw winget output
  including spinner characters; strip like winget_ids does.
- src/tools/mod.rs:456-463 real command for update on Windows is `winget upgrade --all` but Noob runs it with
  `--source winget`; Mac `brew update && brew upgrade` is fine.
- src/tools/mod.rs:253-258 `uninstall rust` says Rust "didn't come from Homebrew or winget"; on Mac it can be
  removed with `rustup self uninstall` - mention it.
- src/tools/mod.rs:160-169 Rust "already installed" only checks PATH; right after the rustup install (before a new
  window) rerunning `install rust` reruns the installer (harmless, idempotent). Also check ~/.cargo/bin.
- catalog.rs:142 Python.Python.3.13 will lag the current Python; pin is fine but note to review on releases.
- Non-curated Mac `real_command` prints `brew install name` even when the package is a cask.
- tests (src/tools/tests.rs): no tests for cask already-installed/not-by-Homebrew, winget uninstall and update,
  Linux (casks refused, ~/.bashrc), `xcode-select` flow (already installed / dialog message), Homebrew install that
  "succeeds" without brew appearing, declining the ~/.zprofile question, `search tools` and `show installed` with
  no Homebrew, `real_command` for `is X installed` with no `programs`. Add these; items 1-6 above would each
  have been caught by one.

## Looks good
Absolute brew path fallback after install (works in the same session); .zprofile edit is asked first and
idempotent; exact-name winget (`--id --exact --source winget`); names starting with "-" refused; real PowerShell
commands for installs are pasteable (override argument is quoted); release workflow, formula and install.ps1
are consistent with README (version 0.3.0 matches the tag example).
