//! The Terminal view: your real shell, drawn by Noob.
//!
//! The pieces, each in its own file:
//!   - `session`: the shell, our parser and our grid, joined together.
//!   - `keys`, `input`: what keys and clipboard events send to the shell.
//!   - `layout`: how many cells fit, and where each one sits.
//!   - `runs`, `palette`, `draw`: painting the grid.
//!   - `select`: choosing text with the mouse, for copying.
//!
//! This file holds the state that lives between frames (where you've scrolled
//! to, what's selected, the cursor's blink) and handles the mouse.

mod draw;
mod input;
mod keys;
mod layout;
mod palette;
mod runs;
mod select;
mod session;

use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, CursorIcon, EventFilter, FontId, MouseWheelUnit, RichText, Sense,
    UiBuilder, vec2,
};

use draw::{CursorLook, Scene};
use input::Request;
use layout::Layout;
pub use palette::Theme;
use select::{Selection, Spot};
use session::Session;

/// Letter size in points. Settings for this come in a later stage.
const FONT_SIZE: f32 = 14.0;
/// Rows are a little taller than the letters, so lines don't touch.
const LINE_SPACING: f32 = 1.2;
/// The cursor is shown for this long, then hidden for this long.
const BLINK: Duration = Duration::from_millis(530);
/// How often to check on the shell while the window isn't focused (Windows
/// doesn't always tell us when the shell ends).
const IDLE_CHECK: Duration = Duration::from_secs(1);
/// The size we start the shell at, before we know how big the window is.
const FIRST_ROWS: usize = 24;
const FIRST_COLS: usize = 80;

enum State {
    Running(Box<Session>),
    /// The shell couldn't even start: why.
    Failed(String),
}

pub struct TerminalView {
    state: Option<State>,
    /// How many lines up from the bottom you've scrolled (0 = following along).
    scroll: usize,
    /// Leftover part of a line from the mouse wheel or trackpad.
    wheel: f32,
    selection: Option<Selection>,
    /// When the blink started over: typing or new output shows the cursor at once.
    blink_from: Instant,
    /// Take the keyboard on the next frame (after the Noob bar typed something).
    wants_focus: bool,
    /// Where the shell starts (a new tab opens in the folder you were in).
    start_folder: Option<std::path::PathBuf>,
    /// Lines typed for you at the start (developer screenshots only).
    #[cfg(feature = "screenshot")]
    demo: Vec<String>,
}

impl TerminalView {
    pub fn new() -> Self {
        TerminalView::starting_in(None)
    }

    /// A terminal whose shell starts in `folder` (or the usual place if `None`).
    pub fn starting_in(folder: Option<std::path::PathBuf>) -> Self {
        TerminalView {
            start_folder: folder,
            state: None,
            scroll: 0,
            wheel: 0.0,
            selection: None,
            blink_from: Instant::now(),
            wants_focus: false,
            #[cfg(feature = "screenshot")]
            demo: demo_lines(),
        }
    }

    /// The title the shell asked for, if any (shown in the top bar).
    pub fn title(&self) -> Option<&str> {
        match &self.state {
            Some(State::Running(session)) if session.ended().is_none() => {
                Some(session.grid().title()).filter(|title| !title.is_empty())
            }
            _ => None,
        }
    }

    fn session(&self) -> Option<&Session> {
        match &self.state {
            Some(State::Running(session)) => Some(session),
            _ => None,
        }
    }

    /// The folder the shell is in right now, if we can tell.
    pub fn shell_folder(&self) -> Option<std::path::PathBuf> {
        self.session().and_then(Session::folder)
    }

    /// Why a command can't be typed into the shell right now, if it can't.
    pub fn busy(&self) -> Option<&'static str> {
        match &self.state {
            Some(State::Running(session)) => session.busy(),
            Some(State::Failed(_)) => Some("The shell couldn't start, so there's nowhere to type."),
            None => Some("The shell is still starting. Try again in a moment."),
        }
    }

    /// Writes `command` into the shell's prompt, without pressing Enter, and
    /// hands the keyboard to the terminal so your Enter runs it.
    pub fn type_into_prompt(&mut self, command: &str) {
        if let Some(State::Running(session)) = &mut self.state {
            session.write(&keys::prompt_bytes(command, cfg!(windows)));
            self.scroll = 0;
            self.blink_from = Instant::now();
            self.wants_focus = true;
        }
    }

    /// Takes in the shell's output while this terminal isn't shown (another tab
    /// is), so it doesn't pile up. True if something new arrived.
    pub fn keep_up(&mut self) -> bool {
        match &mut self.state {
            Some(State::Running(session)) => session.pump(),
            _ => false,
        }
    }

    /// Is a program (like nano, or a download) still running in this shell?
    /// Closing the tab would stop it, so Noob asks first.
    pub fn is_running_program(&self) -> bool {
        self.session()
            .is_some_and(|session| session.ended().is_none() && session.busy().is_some())
    }

    /// Hands the keyboard back to the terminal.
    pub fn focus(&mut self) {
        self.wants_focus = true;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let theme = Theme::for_dark_mode(ui.visuals().dark_mode);
        let font = FontId::monospace(FONT_SIZE);
        let cell = ui.fonts_mut(|fonts| {
            vec2(
                fonts.glyph_width(&font, 'M'),
                (fonts.row_height(&font) * LINE_SPACING).round(),
            )
        });
        let (area, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        ui.painter().rect_filled(area, 0.0, theme.bg);
        let layout = Layout::fit(area, cell);

        // The state is taken out for this frame and put back at the end, so the
        // shell and the rest of `self` can be changed side by side.
        let mut state = match self.state.take() {
            Some(state) => state,
            None => self.start(ui.ctx(), &layout),
        };
        let restart = match &mut state {
            State::Running(session) => {
                self.show_session(ui, session, &response, layout, &theme, font)
            }
            State::Failed(why) => {
                let message = format!("The shell couldn't start: {why}");
                ended_card(ui, area, &theme, &message)
            }
        };
        if restart {
            ui.ctx().request_repaint(); // starts a new shell next frame
        } else {
            self.state = Some(state);
        }
    }

    /// Draws a running (or just ended) shell and handles its keyboard and
    /// mouse. True when you asked for a new shell.
    fn show_session(
        &mut self,
        ui: &mut egui::Ui,
        session: &mut Session,
        response: &egui::Response,
        layout: Layout,
        theme: &Theme,
        font: FontId,
    ) -> bool {
        if session.pump() {
            self.blink_from = Instant::now();
        }
        if (session.grid().rows(), session.grid().cols()) != (layout.rows, layout.cols) {
            session.resize(layout.rows, layout.cols);
        }
        #[cfg(feature = "screenshot")]
        type_demo_lines(session, &mut self.demo);

        // Keep the keyboard here, including Tab, arrows and Escape.
        if std::mem::take(&mut self.wants_focus)
            || response.clicked()
            || ui.memory(|memory| memory.focused().is_none())
        {
            response.request_focus();
        }
        // Typing goes here only while this view has the keyboard and the window
        // itself is the one in front.
        let window_focused = ui.input(|input| input.viewport().focused.unwrap_or(true));
        let focused = response.has_focus() && window_focused;
        if focused && session.ended().is_none() {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    response.id,
                    EventFilter {
                        tab: true,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: true,
                    },
                );
            });
            self.keyboard(ui, session);
        }
        self.mouse(ui, response, &layout, session);

        let grid = session.grid();
        let history = if grid.is_alternate() {
            0
        } else {
            grid.scrollback_len()
        };
        self.scroll = self.scroll.min(history);
        let show_cursor = self.scroll == 0 && grid.cursor_visible() && session.ended().is_none();
        let cursor = match (show_cursor, focused) {
            (false, _) => None,
            (true, false) => Some(CursorLook::Outline),
            (true, true) => blink_is_on(self.blink_from.elapsed()).then_some(CursorLook::Block),
        };
        let area = response.rect;
        let scene = Scene {
            grid,
            layout,
            theme,
            font,
            top_line: grid.scrollback_len() - self.scroll,
            selection: self.selection,
            cursor,
        };
        draw::paint(&ui.painter_at(area), &scene);
        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }
        if self.scroll > 0 {
            scrolled_badge(ui, area, theme, self.scroll);
        }

        if let Some(message) = session.ended().map(str::to_string) {
            return ended_card(ui, area, theme, &message);
        }
        let next = if focused {
            until_next_blink(self.blink_from.elapsed())
        } else {
            IDLE_CHECK
        };
        ui.ctx().request_repaint_after(next);
        false
    }

    fn start(&mut self, ctx: &egui::Context, layout: &Layout) -> State {
        let (rows, cols) = if layout.rows > 1 {
            (layout.rows, layout.cols)
        } else {
            (FIRST_ROWS, FIRST_COLS)
        };
        self.scroll = 0;
        self.selection = None;
        match Session::start(rows, cols, ctx, self.start_folder.as_deref()) {
            Ok(session) => State::Running(Box::new(session)),
            Err(why) => State::Failed(why),
        }
    }

    /// Sends what you type (and pastes) to the shell, and copies the selection.
    fn keyboard(&mut self, ui: &egui::Ui, session: &mut Session) {
        let modes = session.grid().modes();
        let has_selection = self
            .selection
            .is_some_and(|selection| !selection.is_empty());
        let events = ui.input(|input| input.events.clone());
        let requests = input::requests(
            &events,
            modes.app_cursor_keys,
            modes.bracketed_paste,
            has_selection,
            cfg!(target_os = "macos"),
        );
        for request in requests {
            match request {
                Request::Send(bytes) => {
                    session.write(&bytes);
                    // Typing brings you back to the bottom, like other terminals.
                    self.scroll = 0;
                    self.blink_from = Instant::now();
                }
                Request::Copy => {
                    if let Some(selection) = &self.selection {
                        let grid = session.grid();
                        let text =
                            select::selected_text(selection, |line| draw::line_at(grid, line));
                        ui.ctx().copy_text(text);
                    }
                }
                Request::AskForPaste => {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                }
            }
        }
    }

    /// Selecting with a drag, and scrolling with the wheel or trackpad.
    fn mouse(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        layout: &Layout,
        session: &mut Session,
    ) {
        let history = session.grid().scrollback_len();
        let top_line = history - self.scroll.min(history);
        let spot_at = |point| {
            let (row, col) = layout.cell_at(point);
            Spot {
                line: top_line + row,
                col,
            }
        };
        if let Some(point) = response.interact_pointer_pos() {
            if response.drag_started() {
                let spot = spot_at(point);
                self.selection = Some(Selection {
                    anchor: spot,
                    head: spot,
                });
            } else if response.dragged()
                && let Some(selection) = &mut self.selection
            {
                selection.head = spot_at(point);
            }
        }
        if response.clicked() {
            self.selection = None;
        }
        if response.hovered() {
            let lines = wheel_lines(ui, layout);
            self.wheel_by(lines, session);
        }
    }

    /// Moves through history, or (in full-screen programs) presses arrows.
    fn wheel_by(&mut self, lines: f32, session: &mut Session) {
        self.wheel += lines;
        let whole = self.wheel.trunc();
        self.wheel -= whole;
        if whole == 0.0 {
            return;
        }
        let grid = session.grid();
        if !grid.is_alternate() {
            self.scroll = scroll_by(self.scroll, whole, grid.scrollback_len());
            return;
        }
        // Full-screen programs have no history: the wheel moves through them
        // with the arrow keys instead (how less, man and htop expect it).
        let key = if whole > 0.0 {
            egui::Key::ArrowUp
        } else {
            egui::Key::ArrowDown
        };
        let app_cursor = grid.modes().app_cursor_keys;
        if let Some(bytes) = keys::key_bytes(key, egui::Modifiers::NONE, app_cursor) {
            session.write(&bytes.repeat(whole.abs() as usize));
        }
    }
}

/// The card shown over the terminal when the shell has ended (or couldn't
/// start). True when you click "Start a new shell" or press Enter.
fn ended_card(ui: &mut egui::Ui, area: egui::Rect, theme: &Theme, message: &str) -> bool {
    let card = egui::Rect::from_center_size(area.center(), vec2(360.0, 130.0));
    let enter = ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
    let mut clicked = false;
    ui.scope_builder(UiBuilder::new().max_rect(card), |ui| {
        egui::Frame::new()
            .fill(theme.bar)
            .stroke(egui::Stroke::new(1.0, theme.ansi[8]))
            .corner_radius(8.0)
            .inner_margin(16.0)
            .show(ui, |ui| {
                ui.set_width(card.width() - 32.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(message).color(theme.fg).size(15.0));
                    ui.add_space(10.0);
                    let start = egui::Button::new(
                        RichText::new("Start a new shell").color(theme.bg).strong(),
                    )
                    .fill(theme.cursor)
                    .corner_radius(6.0)
                    .min_size(vec2(170.0, 30.0));
                    clicked = ui.add(start).clicked();
                    ui.add_space(4.0);
                    ui.label(RichText::new("or press Enter").color(theme.ansi[8]).small());
                });
            });
    });
    clicked || enter
}

/// How many lines the wheel or trackpad moved this frame (up is positive).
fn wheel_lines(ui: &egui::Ui, layout: &Layout) -> f32 {
    ui.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                egui::Event::MouseWheel { unit, delta, .. } => Some(match unit {
                    MouseWheelUnit::Point => delta.y / layout.cell.y,
                    MouseWheelUnit::Line => delta.y,
                    MouseWheelUnit::Page => delta.y * layout.rows as f32,
                }),
                _ => None,
            })
            .sum()
    })
}

/// "↑ 120 lines up" in the corner while you're looking at history.
fn scrolled_badge(ui: &egui::Ui, area: egui::Rect, theme: &Theme, scroll: usize) {
    let text = format!("↑ {scroll} lines up · type to jump back");
    let painter = ui.painter_at(area);
    let galley = painter.layout_no_wrap(text, FontId::proportional(12.0), theme.bg);
    let size = galley.size() + vec2(16.0, 8.0);
    let rect = Align2::RIGHT_TOP.anchor_size(area.right_top() + vec2(-12.0, 10.0), size);
    painter.rect_filled(rect, 6.0, theme.fg.gamma_multiply(0.85));
    painter.galley(rect.min + vec2(8.0, 4.0), galley, Color32::PLACEHOLDER);
}

/// The cursor shows for one `BLINK`, hides for the next, and so on.
fn blink_is_on(elapsed: Duration) -> bool {
    (elapsed.as_millis() / BLINK.as_millis()).is_multiple_of(2)
}

/// How long until the cursor next appears or disappears.
fn until_next_blink(elapsed: Duration) -> Duration {
    let into = elapsed.as_millis() % BLINK.as_millis();
    BLINK - Duration::from_millis(into as u64)
}

/// The new scroll position after moving `lines` (up is positive), kept
/// between the bottom (0) and the oldest history line.
fn scroll_by(scroll: usize, lines: f32, history: usize) -> usize {
    let moved = scroll as f32 + lines;
    (moved.max(0.0) as usize).min(history)
}

/// Developer screenshots: NOOB_TERMINAL_DEMO="ls -G;htop" types those lines
/// into the shell, a moment after it starts. Never in builds for users.
#[cfg(feature = "screenshot")]
fn demo_lines() -> Vec<String> {
    // Only the first terminal gets them, not every new tab.
    static GIVEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    if GIVEN.swap(true, std::sync::atomic::Ordering::SeqCst) {
        return Vec::new();
    }
    std::env::var("NOOB_TERMINAL_DEMO")
        .unwrap_or_default()
        .split(';')
        .filter(|line| !line.is_empty())
        .rev()
        .map(str::to_string)
        .collect()
}

#[cfg(feature = "screenshot")]
fn type_demo_lines(session: &mut Session, demo: &mut Vec<String>) {
    use std::sync::OnceLock;
    static STARTED: OnceLock<Instant> = OnceLock::new();
    let started = STARTED.get_or_init(Instant::now);
    // Give the shell a moment to show its prompt before typing.
    if started.elapsed() > Duration::from_millis(1500)
        && let Some(line) = demo.pop()
    {
        session.write(format!("{line}\r").as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cursor_blinks_on_then_off() {
        assert!(blink_is_on(Duration::ZERO));
        assert!(!blink_is_on(BLINK + Duration::from_millis(1)));
        assert!(blink_is_on(BLINK * 2));
    }

    #[test]
    fn the_next_blink_is_never_further_than_one_blink_away() {
        assert_eq!(until_next_blink(Duration::ZERO), BLINK);
        assert_eq!(
            until_next_blink(BLINK + Duration::from_millis(30)),
            BLINK - Duration::from_millis(30)
        );
    }

    #[test]
    fn scrolling_stays_between_the_bottom_and_the_oldest_line() {
        assert_eq!(scroll_by(0, 3.0, 100), 3);
        assert_eq!(scroll_by(5, -10.0, 100), 0);
        assert_eq!(scroll_by(95, 10.0, 100), 100);
        assert_eq!(scroll_by(0, 3.0, 0), 0, "nothing to scroll back to yet");
    }
}
