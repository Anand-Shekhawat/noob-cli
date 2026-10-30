# Security review: feat/v3-install (Noob CLI)

Scope: src/tools/*, scripts/install.ps1, .github/workflows/*, scripts/homebrew-formula.sh, history file.
Result: no CRITICAL. 1 HIGH, 5 MEDIUM, several LOW. Not run: install.ps1, real installs.

## HIGH

H1. src/tools/package.rs:~172 (valid_name) allows `/`, so `install someone/tap/tool` passes.
Homebrew treats `user/repo/formula` as a third-party tap: `brew info` and `brew install` will
auto-tap github.com/user/homebrew-repo and run Ruby from it with the user's rights. The prompt
(mod.rs:~84) says "from Homebrew's catalog", which is false for taps, so the yes/no consent is
misleading. Risk is a user pasting an attacker-provided "install x/y/z" line (social engineering),
not remote exploitation. Fix: reject names with more than zero `/` for Homebrew (allow `/` never,
or an explicit allowlist like homebrew/cask-versions), and for winget (IDs use `.`, not `/`).
Also reject names starting with `.` (e.g. `./x.rb`; brew refuses path formulae today, but do not rely on it).
Argument/option injection otherwise is handled: leading `-` rejected, no whitespace/quote/`=` chars,
`install` args are `["install", name]` with no shell. `--` before the name would be cheap defence in depth
(brew supports it; winget: not needed since `--id` takes the value).

## MEDIUM

M1. src/tools/package.rs:~7 (HOMEBREW_PLACES) + system.rs find_in: `find("brew")` trusts PATH first. A hostile
earlier PATH entry (or a cwd entry like `.`/empty PATH element on Unix, which split_paths yields as "")
runs an attacker `brew` with the user's rights, including `sudo` prompts later. Prefer the fixed
absolute locations first, PATH second; ignore empty/relative PATH entries. Same for winget on Windows
(only .exe/.cmd/.bat in user-writable PATH folders; prefer `%LOCALAPPDATA%\Microsoft\WindowsApps\winget.exe`).
Note Windows `Command::new(path)` with a full path is fine; no cwd lookup because find returns full paths.
M2. src/tools/system.rs:~66 `/bin/bash` is hardcoded (good). Script installers (catalog.rs:73, :78) are
constants, run via `bash -c <constant>` with no user data interpolated. Verified: Package::Script only
comes from catalog constants (package.rs for_tool); no path from user text. OK. Residual: the curl|sh
pattern trusts raw.githubusercontent.com HEAD (Homebrew) and sh.rustup.rs, i.e. the vendors' own channel.
Accepted risk; Homebrew URL uses `HEAD` (moving) by vendor's own instruction. The rustup line uses
`-y` after a Noob yes/no, fine. Suggest the confirmation text show the URL (real_install already returns it).
M3. src/tools/setup.rs:~58 add_line: appends via OpenOptions to ~/.zprofile / ~/.bashrc, which follows
symlinks and writes through them. If the profile is a symlink to a dotfiles repo it edits that target (benign but surprising);
a malicious symlink in $HOME would already imply compromise. Content is `eval "$(<brew path> shellenv)"`
where path is one of three constants or a PATH-found brew (see M1): a PATH-found path containing `"`, `$(`
or backticks would be injected into the user's profile and executed on every shell start. Fix: only write the
line when brew is one of HOMEBREW_PLACES (constants), or reject paths with non-[A-Za-z0-9/._-] chars.
Idempotence: `contains(&line)` is fine (substring match, so a commented-out line counts as present: LOW).
Also asks consent first; no trailing-newline check on existing file (line could glue to a last line lacking "\n": the
writeln starts with "\n" so OK).
M4. .github/workflows/*.yml: third-party actions are only first-party `actions/checkout@v4` (mutable tag, not SHA).
Release job has `contents: write` for the whole workflow, including the build/test jobs that run `cargo build`
(build.rs/dependency code executes with a write token; checkout persists the token in .git config by default).
Fix: top-level `permissions: contents: read`, grant `write` only on the create-release and upload steps/jobs;
add `persist-credentials: false`; pin actions to commit SHAs (Dependabot for github-actions).
M5. Artifact integrity (release.yml, install.ps1:20-33, homebrew-formula.sh:12-15): the `.sha256` is
downloaded from the same release as the zip, so it only detects corruption/truncation, not tampering: anyone who can
replace the zip (stolen token, compromised repo) replaces the checksum too. TLS to github.com protects in transit only.
What adds real value: (a) Homebrew formula: the sha256 pinned in a separate tap repo is a genuine second
channel, so generate it from the CI-built file and commit via reviewed PR (script fetches it from the same release
= no gain at generation time, but gain at user time); (b) GitHub artifact attestations
(`actions/attest-build-provenance`, free for public repos) and `gh attestation verify`, (c) embed the expected
hash in the script/README of a separate signed commit, (d) Windows: Authenticode signing (accepted risk, owner chose free).
The honest wording in install.ps1 "checks the download isn't damaged" is accurate; keep it.
Also: `--clobber` upload (release.yml:~64) lets a re-run overwrite published assets; consider no clobber and immutable releases.

## LOW

L1. .github/workflows/release.yml:~16,~64,~89: `${{ github.ref_name }}` is interpolated directly into `run:` scripts (shell and pwsh).
Tag names may contain `$(...)`, quotes, backticks; pushing a tag needs write access so exploitability is low, but fix by
passing via `env: TAG: ${{ github.ref_name }}` and using "$TAG" / $env:TAG. Line ~89 also builds a regex from the tag
(`-notmatch '...'.TrimStart('v')` : version "0.3.0" as regex, dots are wildcards; use -notlike or [regex]::Escape). Trigger `tags: ["v*"]`
accepts any `v...` tag; add semver validation step. `${{ matrix.target }}` is a fixed constant, safe.
L2. release.yml try-windows-installer runs ./scripts/install.ps1 from the checkout, which downloads `releases/latest`
(not this tag) via the API: it can test a different release than the one just built, and a draft/pre-release race. Pass the tag or
mirror the logic. Also the job runs after build only with needs: build but not on failure of one matrix leg (fail-fast false): the
release can exist half-published (create-release runs first, public immediately). Consider `--draft` then publish at the end.
L3. scripts/install.ps1: Source is api.github.com + browser_download_url over HTTPS (Windows PowerShell 5.1 may default to TLS 1.0/1.1
on old systems; add `[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12`). The script itself is fetched from the
`main` branch (mutable) and piped to iex: an attacker with push to main controls it, normal for this pattern; document or pin to a tag.
Asset is matched by wildcard `*-$target.zip` and downloaded URL is taken from the API response with no host check; add a check that
`browser_download_url` starts with `https://github.com/$repo/releases/download/`. Temp dir is a random GUID under the user's temp (no
predictable-name race), cleaned in `finally`. Expand-Archive then `Get-ChildItem -Recurse -Filter noob.exe | First 1`: the zip is
trusted (post checksum), but zip-slip is handled by Expand-Archive in recent versions; fine. `$installDir` is defined inside the try-less
flow after `finally`: if the download throws, the PATH code is never reached (good, `Stop`). Failure mode: a running noob.exe makes Copy-Item fail
(clear error). PATH edit: user scope only, deduped by exact string match (case-sensitive `-notcontains` is actually case-insensitive in PS; trailing
slash variants would duplicate: cosmetic). Empty-string PATH handled. A user-writable folder at the front of nothing: it is appended (not prepended), good
(no shadowing of system tools). `$installDir` is under %LOCALAPPDATA% so only the user can write: acceptable.
L4. winget `--override` (catalog.rs:~171): constant string, no user data; passed as a single argv element, so no splitting by Rust.
Note winget/VS installer parse the override themselves; fine. `--source winget` pinned for exact installs (good, avoids msstore/other-source
ID confusion). Missing: `--accept-package-agreements` is not forced (good: user sees terms on screen).
L5. Trusting search output (package.rs search/brew_names/winget_ids): output is only displayed as suggestions and later goes
back through `valid_name` when the user retypes (mod.rs:~427). Terminal escape sequences in search results (brew/winget text printed raw)
could manipulate the screen; strip control chars (`c.is_control()`) before printing. `exact_package` info check followed by install is a TOCTOU of
no practical value.
L6. src/repl.rs:~57-66 history: `~/.noob_history` is saved with default umask (rustyline creates 0600 on Unix in recent versions; verify) and
stores everything typed except lines starting with a space (repl.rs:138). That includes `write "secret" to notes.txt`-style text, and any
`install`-related text. Document it in README (leading space = not saved, `NOOB_HISTORY=off`). `NOOB_HISTORY` lets whoever controls the
environment redirect the file to any path the user can write (append/overwrite via save_history truncates and rewrites: a poisoned env var
could clobber an arbitrary user file with history text, e.g. `NOOB_HISTORY=~/.zshrc` would overwrite it). Low (attacker needs env control, which
already implies code execution), but cheap hardening: refuse paths that exist and are not previously a Noob history file, or ignore the var unless it is `off`.
Windows ACLs on the history file are inherited from the profile (user-only by default).
L7. scripts/homebrew-formula.sh: `set -euo pipefail` is set, but in `checksum()` a failed `curl` inside `$(...)` within a heredoc does not abort
(command substitution failure in heredoc expansion is not fatal), so an empty `sha256 ""` could be written; also `cut` on an HTML error yields junk.
Validate the result with a regex `^[0-9a-f]{64}$` and exit non-zero. Version tag argument is interpolated into URLs and Ruby source: validate
`^v[0-9]+\.[0-9]+\.[0-9]+$` (a tag with `"` could inject Ruby into the formula). Formula `test` is sound. Linux ARM/aarch64 not offered (functional, not security).
L8. ci.yml: `push:` on all branches with `contents: read` is good. No `pull_request` trigger, so forks do not run with secrets (good; but
fork PRs get no CI). No `cargo audit`/`cargo deny` step, Cargo.lock should be committed and `--locked` used in tests too (build uses it).

## Accepted risk
No code signing / notarization: Windows SmartScreen and macOS Gatekeeper warnings, no publisher identity. Noted only.

## Positives
No shell for user input anywhere; Command with argv; valid_name blocks leading `-`, spaces, quotes, `$`, `;`, `:`; consent before every install;
uninstall/check pinned to `--exact`; setup asks before touching profile; workflows use read-only default on CI, token via env not inline in the gh calls.
