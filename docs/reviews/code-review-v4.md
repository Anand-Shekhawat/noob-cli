# Code review v4: feat/app (window + handoff)

Verified: `cargo test --features app` passes; `cargo clippy --all-targets --features app -- -D warnings` clean.
Terminal regression vs main: none found (same messages, streams, exit codes; Ask/declined/Bye/Clear paths match).
Window flow: no silent "no" found. The only mid-work `ask` is `tools/setup.rs:45` (after Homebrew install), and install is handed off to a terminal. Delete/Write questions go through Yes/No buttons and are not re-asked on Yes.
Handoff security: the .command script is sound (create_new, 0700, every word single-quoted, self-deletes; temp dir is per-user on macOS). Windows args are quoted by Rust std; no injection found.

## CRITICAL
None.

## HIGH
None.

## MEDIUM
1. scripts/install.ps1:49-53 - missing files are skipped silently (`if ($file)`). If noob.exe is absent from the zip, install still prints "installed" (main used to fail). Fail when noob.exe is missing; only noob-app.exe/noob.ico may be optional.
2. src/chat.rs:~155 (`run` -> `engine::finish`) - every command runs on the UI thread. Window now starts in $HOME, so `find text ...`, `show size`, `show installed`/`search tools` (spawn brew/winget) freeze the window with no feedback. Run on a thread with a "working..." entry, or document the limit.

## LOW
3. src/terminal.rs:28-34 - if the sibling `noob` is not found it falls back to bare `noob`, resolved through PATH (and on Windows CreateProcess search order) and run in a terminal that may ask for a password. Prefer returning an error.
4. src/terminal.rs:46-65 - if `open` fails, the .command file is left in the temp dir (never self-deletes). Remove it on error.
5. src/terminal.rs:90-115 - cwd differs by OS: Mac script does `cd ~`, Windows/Linux inherit the window's cwd. Harmless for installs, but inconsistent.
6. src/main.rs:30-36 - pause is triggered by an env var that child processes inherit; a leftover NOOB_PAUSE_AT_END in a user's shell makes every `noob <phrase>` wait for Enter. A hidden flag would be safer.
7. Tests: no test for `command_file` (create_new refusal, 0700, file content), none for the Windows/Linux `open`, none for chat `answer(true)` on Write/Delete, or `go into` inside the window. The script-quoting test is good.
8. scripts/make-mac-app.sh:30-42 - Info.plist lacks CFBundleInfoDictionaryVersion; scripts/homebrew-cask.sh has no `depends_on macos: ">= :big_sur"` although LSMinimumSystemVersion is 11.0.
9. README / scripts/install.ps1:1-8 - install.ps1 header still says it only installs noob.exe; README says nothing about Linux having no window, and the file table omits assets/ and scripts/. Cask installs the window only, not `noob` on PATH (not claimed, but may surprise).

## Packaging checks (OK)
Release asset `Noob-$TAG-$TARGET.zip(.sha256)` matches the cask URL `Noob-v#{version}-#{arch}-apple-darwin.zip` and its checksum fetch. install.ps1 pattern `*-x86_64-pc-windows-msvc.zip` matches the Windows zip, which now contains noob-app.exe and noob.ico. Ad-hoc codesign runs before `ditto --keepParent`, so the zip keeps the signature. `--features ""` for Linux is valid. Tag/version validated in scripts.
