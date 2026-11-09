//! Teaching tests: real bytes that real programs print go through the stage 2
//! parser and into the grid, then we check what the screen would show.

use super::*;
use crate::term::parser::Parser;

/// A small screen with the shell's bytes already fed in.
fn screen(rows: usize, cols: usize, bytes: &[u8]) -> Grid {
    let mut grid = Grid::new(rows, cols);
    feed(&mut grid, bytes);
    grid
}

/// Runs bytes through a fresh parser into the grid, like Noob's window will.
fn feed(grid: &mut Grid, bytes: &[u8]) {
    for action in Parser::new().feed(bytes) {
        grid.apply(&action);
    }
}

/// The screen as plain text, one string per row (trailing spaces dropped).
fn text(grid: &Grid) -> Vec<String> {
    grid.screen_text()
}

/// Shorthand for the expected text of a screen.
fn lines(rows: &[&str]) -> Vec<String> {
    rows.iter().map(|row| row.to_string()).collect()
}

fn look_at(grid: &Grid, row: usize, col: usize) -> Look {
    grid.cell(row, col).expect("cell is on the screen").look
}

// ── Printing, wrapping and the simple control codes ────────────────────────

#[test]
fn printing_puts_letters_on_the_screen_and_moves_the_cursor() {
    let grid = screen(3, 10, b"hi");
    assert_eq!(text(&grid), lines(&["hi", "", ""]));
    assert_eq!(grid.cursor(), (0, 2));
    assert!(grid.cursor_visible());
}

#[test]
fn enter_is_carriage_return_plus_line_feed() {
    // The shell sends "\r\n" for a new line: back to the start, then down.
    let grid = screen(3, 10, b"one\r\ntwo");
    assert_eq!(text(&grid), lines(&["one", "two", ""]));
    assert_eq!(grid.cursor(), (1, 3));
}

#[test]
fn a_long_line_wraps_onto_the_next_row() {
    let grid = screen(3, 5, b"abcdefgh");
    assert_eq!(text(&grid), lines(&["abcde", "fgh", ""]));
    // The first row remembers that it carried on (copy/paste will need this).
    assert!(grid.row(0).unwrap().wrapped);
    assert!(!grid.row(1).unwrap().wrapped);
}

#[test]
fn filling_the_last_column_waits_before_wrapping() {
    // After the 5th letter the cursor stays on the last column. Only the next
    // letter wraps, so "\r\n" right after a full row doesn't leave a blank row.
    let grid = screen(3, 5, b"abcde\r\nx");
    assert_eq!(text(&grid), lines(&["abcde", "x", ""]));
}

#[test]
fn backspace_steps_left_and_the_next_letter_overwrites() {
    let grid = screen(1, 10, b"cat\x08\x08u");
    assert_eq!(text(&grid), lines(&["cut"]));
    // Backspace at the left edge stays put.
    let grid = screen(1, 10, b"\x08\x08x");
    assert_eq!(text(&grid), lines(&["x"]));
}

#[test]
fn tab_jumps_to_the_next_multiple_of_eight() {
    let grid = screen(1, 20, b"a\tb\tc");
    assert_eq!(text(&grid), lines(&["a       b       c"]));
    // At the end of the line a tab stops on the last column.
    let grid = screen(1, 20, b"\t\t\t\tz");
    assert_eq!(grid.cursor(), (0, 19));
    assert_eq!(text(&grid), lines(&["                   z"]));
}

// ── Real programs ──────────────────────────────────────────────────────────

#[test]
fn a_colour_ls_shows_a_bold_blue_folder_and_a_plain_file() {
    let grid = screen(2, 30, b"\x1b[0m\x1b[01;34mDocuments\x1b[0m  notes.txt\r\n");
    assert_eq!(text(&grid), lines(&["Documents  notes.txt", ""]));

    let folder = look_at(&grid, 0, 0);
    assert_eq!(folder.fg, Color::Indexed(4)); // 34 = blue
    assert!(folder.bold);

    let file = look_at(&grid, 0, 11);
    assert_eq!(file, Look::default());
    assert_eq!(grid.cursor(), (1, 0));
}

#[test]
fn clear_empties_the_screen_and_the_history_and_goes_home() {
    let mut grid = screen(2, 10, b"one\r\ntwo\r\nthree\r\nfour");
    assert_eq!(grid.scrollback_len(), 2);
    // What `clear` prints on a Mac: erase history, cursor home, erase screen.
    feed(&mut grid, b"\x1b[3J\x1b[H\x1b[2J");
    assert_eq!(text(&grid), lines(&["", ""]));
    assert_eq!(grid.scrollback_len(), 0);
    assert_eq!(grid.cursor(), (0, 0));
}

#[test]
fn lines_that_scroll_off_the_top_go_into_the_history() {
    let grid = screen(2, 10, b"one\r\ntwo\r\nthree\r\nfour");
    assert_eq!(text(&grid), lines(&["three", "four"]));
    assert_eq!(grid.scrollback_text(), lines(&["one", "two"]));
}

#[test]
fn the_history_has_a_size_limit() {
    let mut grid = Grid::new(1, 10);
    grid.set_scrollback_limit(3);
    feed(&mut grid, b"1\r\n2\r\n3\r\n4\r\n5\r\n6");
    // Only the newest 3 lines are kept; the oldest are forgotten.
    assert_eq!(grid.scrollback_text(), lines(&["3", "4", "5"]));
}

#[test]
fn nano_uses_the_alternate_screen_and_leaves_the_shell_screen_untouched() {
    let mut grid = screen(3, 20, b"$ ls\r\nnotes.txt\r\n$ nano");
    // nano switches to the alternate screen, draws, then scrolls a lot.
    feed(
        &mut grid,
        b"\x1b[?1049h\x1b[H\x1b[2J  GNU nano\r\n\r\n^X Exit",
    );
    assert!(grid.is_alternate());
    assert_eq!(text(&grid), lines(&["  GNU nano", "", "^X Exit"]));
    feed(&mut grid, b"\r\nmore\r\nmore\r\nmore");
    // The alternate screen has no history.
    assert_eq!(grid.scrollback_len(), 0);

    // Leaving puts the shell's screen and cursor back exactly as they were.
    feed(&mut grid, b"\x1b[?1049l");
    assert!(!grid.is_alternate());
    assert_eq!(text(&grid), lines(&["$ ls", "notes.txt", "$ nano"]));
    assert_eq!(grid.cursor(), (2, 6));
}

#[test]
fn the_window_title_comes_from_osc() {
    let grid = screen(1, 10, b"\x1b]2;anand@mac: ~\x07");
    assert_eq!(grid.title(), "anand@mac: ~");
    let grid = screen(1, 10, b"\x1b]0;htop\x1b\\");
    assert_eq!(grid.title(), "htop");
}

// ── Cursor moves ───────────────────────────────────────────────────────────

#[test]
fn cup_moves_to_a_row_and_column_counting_from_one() {
    let grid = screen(5, 10, b"\x1b[3;4HX");
    assert_eq!(grid.cell(2, 3).unwrap().letter, 'X');
    // No numbers means "home" (top left).
    let grid = screen(5, 10, b"abc\x1b[H");
    assert_eq!(grid.cursor(), (0, 0));
}

#[test]
fn cursor_moves_stop_at_the_edges() {
    let grid = screen(5, 10, b"\x1b[99;99H");
    assert_eq!(grid.cursor(), (4, 9));
    let grid = screen(5, 10, b"\x1b[3;3H\x1b[9A\x1b[9D");
    assert_eq!(grid.cursor(), (0, 0));
    let grid = screen(5, 10, b"\x1b[2B\x1b[3C");
    assert_eq!(grid.cursor(), (2, 3));
}

#[test]
fn column_and_row_jumps() {
    // CHA: column 5. VPA: row 3. CNL: down 1 and to the start.
    let grid = screen(5, 10, b"\x1b[5G");
    assert_eq!(grid.cursor(), (0, 4));
    let grid = screen(5, 10, b"\x1b[5G\x1b[3d");
    assert_eq!(grid.cursor(), (2, 4));
    let grid = screen(5, 10, b"\x1b[5G\x1b[E");
    assert_eq!(grid.cursor(), (1, 0));
}

#[test]
fn save_and_restore_bring_back_the_cursor_and_its_colours() {
    let grid = screen(5, 10, b"\x1b[2;2H\x1b[31m\x1b7\x1b[H\x1b[0m\x1b8X");
    assert_eq!(grid.cell(1, 1).unwrap().letter, 'X');
    assert_eq!(look_at(&grid, 1, 1).fg, Color::Indexed(1));
    // The CSI spelling works too.
    let grid = screen(5, 10, b"\x1b[3;3H\x1b[s\x1b[H\x1b[u");
    assert_eq!(grid.cursor(), (2, 2));
}

#[test]
fn hiding_and_showing_the_cursor() {
    let grid = screen(1, 10, b"\x1b[?25l");
    assert!(!grid.cursor_visible());
    let grid = screen(1, 10, b"\x1b[?25l\x1b[?25h");
    assert!(grid.cursor_visible());
}

// ── Erasing, inserting and deleting ────────────────────────────────────────

#[test]
fn erase_in_line_clears_part_or_all_of_the_row() {
    let grid = screen(1, 10, b"abcdef\x1b[4G\x1b[K");
    assert_eq!(text(&grid), lines(&["abc"]));
    let grid = screen(1, 10, b"abcdef\x1b[4G\x1b[1K");
    assert_eq!(text(&grid), lines(&["    ef"]));
    let grid = screen(1, 10, b"abcdef\x1b[2K");
    assert_eq!(text(&grid), lines(&[""]));
}

#[test]
fn erase_in_display_clears_below_or_above_the_cursor() {
    let start = b"aaa\r\nbbb\r\nccc\x1b[2;2H";
    let mut grid = screen(3, 5, start);
    feed(&mut grid, b"\x1b[J");
    assert_eq!(text(&grid), lines(&["aaa", "b", ""]));
    let mut grid = screen(3, 5, start);
    feed(&mut grid, b"\x1b[1J");
    assert_eq!(text(&grid), lines(&["", "  b", "ccc"]));
}

#[test]
fn erasing_uses_the_current_background_colour() {
    // Programs like htop paint a coloured bar by setting a background and erasing.
    let grid = screen(1, 4, b"\x1b[44m\x1b[2K");
    assert_eq!(look_at(&grid, 0, 3).bg, Color::Indexed(4));
}

#[test]
fn insert_and_delete_characters_shift_the_rest_of_the_line() {
    let grid = screen(1, 10, b"abcdef\x1b[3G\x1b[2@");
    assert_eq!(text(&grid), lines(&["ab  cdef"]));
    let grid = screen(1, 10, b"abcdef\x1b[3G\x1b[2P");
    assert_eq!(text(&grid), lines(&["abef"]));
    // ECH blanks without shifting.
    let grid = screen(1, 10, b"abcdef\x1b[3G\x1b[2X");
    assert_eq!(text(&grid), lines(&["ab  ef"]));
}

#[test]
fn insert_and_delete_lines_push_rows_down_or_pull_them_up() {
    let start = b"1\r\n2\r\n3\r\n4\x1b[2;1H";
    let mut grid = screen(4, 5, start);
    feed(&mut grid, b"\x1b[L");
    assert_eq!(text(&grid), lines(&["1", "", "2", "3"]));
    let mut grid = screen(4, 5, start);
    feed(&mut grid, b"\x1b[2M");
    assert_eq!(text(&grid), lines(&["1", "4", "", ""]));
}

// ── Scrolling and scroll regions ───────────────────────────────────────────

#[test]
fn scroll_up_and_down_move_the_whole_screen() {
    let mut grid = screen(3, 5, b"1\r\n2\r\n3");
    feed(&mut grid, b"\x1b[S");
    assert_eq!(text(&grid), lines(&["2", "3", ""]));
    assert_eq!(grid.scrollback_text(), lines(&["1"]));
    feed(&mut grid, b"\x1b[2T");
    assert_eq!(text(&grid), lines(&["", "", "2"]));
}

#[test]
fn a_scroll_region_keeps_the_top_and_bottom_rows_still() {
    // Like `less` or vim: a title on row 1, a status bar on row 4, and only the
    // middle scrolls. Lines scrolled inside a region don't go into the history.
    let mut grid = screen(4, 10, b"title\r\na\r\nb\r\nstatus");
    feed(&mut grid, b"\x1b[2;3r");
    assert_eq!(grid.cursor(), (0, 0)); // Setting a region sends the cursor home.
    feed(&mut grid, b"\x1b[3;1H\nc");
    assert_eq!(text(&grid), lines(&["title", "b", "c", "status"]));
    assert_eq!(grid.scrollback_len(), 0);
}

#[test]
fn reverse_index_at_the_top_scrolls_down() {
    let grid = screen(3, 5, b"1\r\n2\r\n3\x1b[H\x1bM");
    assert_eq!(text(&grid), lines(&["", "1", "2"]));
}

// ── Colours and styles (SGR) ───────────────────────────────────────────────

#[test]
fn sgr_sets_styles_and_zero_resets_them() {
    let grid = screen(1, 10, b"\x1b[1;2;3;4;7mA\x1b[0mB");
    let a = look_at(&grid, 0, 0);
    assert!(a.bold && a.dim && a.italic && a.underline && a.inverse);
    assert_eq!(look_at(&grid, 0, 1), Look::default());
    // `ESC[m` with no numbers is a reset too.
    let grid = screen(1, 10, b"\x1b[1m\x1b[mB");
    assert_eq!(look_at(&grid, 0, 0), Look::default());
}

#[test]
fn sgr_turns_single_styles_off() {
    let grid = screen(1, 10, b"\x1b[1;3;4;7m\x1b[22;23;24;27mA");
    assert_eq!(look_at(&grid, 0, 0), Look::default());
}

#[test]
fn the_16_basic_colours() {
    let grid = screen(1, 10, b"\x1b[31;42mA\x1b[91;102mB\x1b[39;49mC");
    assert_eq!(look_at(&grid, 0, 0).fg, Color::Indexed(1));
    assert_eq!(look_at(&grid, 0, 0).bg, Color::Indexed(2));
    assert_eq!(look_at(&grid, 0, 1).fg, Color::Indexed(9)); // bright red
    assert_eq!(look_at(&grid, 0, 1).bg, Color::Indexed(10)); // bright green
    assert_eq!(look_at(&grid, 0, 2), Look::default());
}

#[test]
fn the_256_colours_and_true_colours() {
    let grid = screen(1, 10, b"\x1b[38;5;208mA\x1b[48;2;10;20;30mB");
    assert_eq!(look_at(&grid, 0, 0).fg, Color::Indexed(208));
    assert_eq!(look_at(&grid, 0, 1).bg, Color::Rgb(10, 20, 30));
    // The colon spellings, with and without the "colour space" slot.
    let grid = screen(
        1,
        10,
        b"\x1b[38:2:255:0:0mA\x1b[38:2::0:255:0mB\x1b[48:5:17mC",
    );
    assert_eq!(look_at(&grid, 0, 0).fg, Color::Rgb(255, 0, 0));
    assert_eq!(look_at(&grid, 0, 1).fg, Color::Rgb(0, 255, 0));
    assert_eq!(look_at(&grid, 0, 2).bg, Color::Indexed(17));
}

#[test]
fn a_cut_off_colour_command_does_not_break_the_ones_after_it() {
    let grid = screen(1, 10, b"\x1b[38;5mA\x1b[1;38;2;1mB");
    assert_eq!(look_at(&grid, 0, 0).fg, Color::Default);
    assert!(look_at(&grid, 0, 1).bold);
}

// ── Wide letters (emoji and CJK) ───────────────────────────────────────────

#[test]
fn emoji_and_chinese_letters_take_two_cells() {
    let grid = screen(1, 10, "a世b👋c".as_bytes());
    assert_eq!(text(&grid), lines(&["a世b👋c"]));
    assert_eq!(grid.cell(0, 1).unwrap().width, Width::Two);
    assert_eq!(grid.cell(0, 2).unwrap().width, Width::Covered);
    assert_eq!(grid.cursor(), (0, 7));
}

#[test]
fn a_wide_letter_that_does_not_fit_wraps_whole() {
    let grid = screen(2, 4, "abc世".as_bytes());
    assert_eq!(text(&grid), lines(&["abc", "世"]));
}

#[test]
fn overwriting_half_a_wide_letter_removes_all_of_it() {
    let grid = screen(1, 6, "世界\x1b[2Gx".as_bytes());
    assert_eq!(text(&grid), lines(&[" x界"]));
}

// ── Resizing ───────────────────────────────────────────────────────────────

#[test]
fn growing_the_window_adds_blank_space() {
    let mut grid = screen(2, 4, b"ab\r\ncd");
    grid.resize(3, 6);
    assert_eq!((grid.rows(), grid.cols()), (3, 6));
    assert_eq!(text(&grid), lines(&["ab", "cd", ""]));
}

#[test]
fn shrinking_the_window_keeps_the_cursor_row_and_saves_the_top_in_history() {
    let mut grid = screen(4, 10, b"1\r\n2\r\n3\r\n4");
    grid.resize(2, 10);
    assert_eq!(text(&grid), lines(&["3", "4"]));
    assert_eq!(grid.scrollback_text(), lines(&["1", "2"]));
    assert_eq!(grid.cursor(), (1, 1));
    // Growing again brings the history back onto the screen.
    grid.resize(4, 10);
    assert_eq!(text(&grid), lines(&["1", "2", "3", "4"]));
    assert_eq!(grid.cursor(), (3, 1));
}

#[test]
fn narrowing_cuts_rows_and_never_leaves_half_a_wide_letter() {
    let mut grid = screen(1, 10, "abc世".as_bytes());
    grid.resize(1, 4);
    assert_eq!(text(&grid), lines(&["abc"]));
    assert_eq!(grid.cursor(), (0, 3));
}

#[test]
fn a_zero_sized_window_is_treated_as_one_cell() {
    let mut grid = screen(3, 3, b"abc");
    grid.resize(0, 0);
    assert_eq!((grid.rows(), grid.cols()), (1, 1));
    feed(&mut grid, "x世\x1b[5;5H\x1b[3@\x1b[2L".as_bytes());
}

// ── Answers the terminal sends back ────────────────────────────────────────

#[test]
fn the_grid_answers_where_is_your_cursor() {
    // ConPTY on Windows asks this before starting the shell (see `pty.rs`).
    let mut grid = screen(5, 10, b"\x1b[3;7H\x1b[6n");
    assert_eq!(grid.take_replies(), b"\x1b[3;7R".to_vec());
    assert!(grid.take_replies().is_empty());
}

#[test]
fn modes_the_keyboard_will_need_are_remembered() {
    let grid = screen(1, 10, b"\x1b[?1h\x1b[?2004h");
    assert!(grid.modes().app_cursor_keys);
    assert!(grid.modes().bracketed_paste);
}

// ── Never crash ────────────────────────────────────────────────────────────

#[test]
fn huge_and_odd_numbers_are_clamped_or_ignored() {
    let mut grid = Grid::new(3, 5);
    feed(
        &mut grid,
        b"\x1b[65535;65535H\x1b[65535@\x1b[65535P\x1b[65535L\x1b[65535M\
          \x1b[65535S\x1b[65535T\x1b[65535X\x1b[65535b\x1b[9;2r\x1b[0;0r\
          \x1b[99J\x1b[99K\x1b[?9999h\x1b[38;5;999m\x1b[38;2;999;999;999mZ\
          \x1b[65535I\x1b[65535Z\x1b[65535E\x1b[65535F\x1bc",
    );
    assert_eq!(grid.cursor(), (0, 0));
}

#[test]
fn random_bytes_never_crash_the_grid() {
    // A tiny repeatable "random number" maker, so this test is the same each run.
    let mut seed: u64 = 42;
    let mut grid = Grid::new(6, 12);
    let mut parser = Parser::new();
    let pieces: [&[u8]; 8] = [
        b"\x1b[",
        b";",
        b"H",
        b"\x1b[?1049h",
        b"\x1b[?1049l",
        "世👋".as_bytes(),
        b"\r\n",
        b"m",
    ];
    for round in 0..20_000_u32 {
        seed = seed
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let roll = (seed >> 33) as usize;
        let bytes: Vec<u8> = if roll.is_multiple_of(3) {
            pieces[roll % pieces.len()].to_vec()
        } else {
            vec![(roll % 256) as u8, b'0' + (roll % 10) as u8]
        };
        for action in parser.feed(&bytes) {
            grid.apply(&action);
        }
        if round.is_multiple_of(997) {
            grid.resize(1 + roll % 9, 1 + roll % 15);
        }
        let (row, col) = grid.cursor();
        assert!(row < grid.rows() && col < grid.cols());
    }
}
