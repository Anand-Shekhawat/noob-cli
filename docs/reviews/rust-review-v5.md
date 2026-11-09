# Rust review v5: src/term/grid (branch feat/terminal-grid)

Verdict: APPROVE. No CRITICAL or HIGH findings.

## Panic safety (verified)
Read every index/slice/subtraction in mod.rs, csi.rs, resize.rs, cell.rs, sgr.rs.
All are guarded (cursor always < rows/cols, tabs.len()==cols, rows/cols clamped to 1..=1000,
`count(n)-1` safe because count>=1, drain/splice bounds clamped, fit_rows off_the_top <= keep).
Throwaway fuzz (in a COPY under the scratchpad, repo untouched): 3000 grids (1..12 x 1..12), 300
steps each, mixing ~55 sequences (wide chars, ICH/DCH/ECH/IL/DL, DECSTBM incl. out of range,
alt screen 47/1047/1049, save/restore, DECOM, DECAWM, insert mode, REP, tabs, ESC c, DSR,
65535 params) plus random resizes and scrollback-limit changes. Result: no panic; cursor always
on screen; every row always exactly `cols` cells.

## Wide characters (verified)
print() writes both halves then repair_wide(col-1, col+width+1); overwrite of either half, ECH, EL,
DCH, ICH, insert mode and resize all repair orphan halves. A wide char that does not fit wraps whole.

## MEDIUM (not blocking)
- csi.rs:`'@'` (ICH) via insert_blanks does not clear `pending_wrap` (DCH does). After ICH at the
  last column the next letter wraps instead of overwriting. Input: fill row, `ESC[@`, `x`.
- csi.rs 1049 `l` while not on the alternate screen still runs restore_cursor: cursor jumps home if
  nothing saved. Input: `abc` then `ESC[?1049l`.
- mod.rs `replies` grows without bound if the owner never drains it (`ESC[6n` repeated). Cap it.
- resize.rs: when the alternate screen is shown and no saved cursor exists (mode 47), the main
  screen is fitted using the alt cursor row (only affects which rows go to history).
- csi.rs ED 2 / ED 0 whole-row paths do not clear `pending_wrap` (EL does); xterm behaves the same
  or close, low impact.
