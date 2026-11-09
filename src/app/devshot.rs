//! For developers only (the `screenshot` feature): saves a picture of the
//! window a few seconds after it opens, then closes it. eframe's own
//! `EFRAME_SCREENSHOT_TO` snaps the second frame, before the shell has printed
//! anything; this waits.
//!
//!   NOOB_SHOT_TO=shot.ppm NOOB_SHOT_AFTER_MS=4000 \
//!     cargo run --features screenshot --bin noob-app
//!
//! `NOOB_KEYS` presses keys for you, one piece per frame from 2.5 seconds in,
//! so the real keyboard path can be checked too. Pieces are split by `|`:
//! `text:echo hi` types text, `key:Enter` presses a key, `ctrl:C` holds Ctrl,
//! `cmd:K` holds Cmd (Ctrl on Windows and Linux),
//! `wheel:5` turns the mouse wheel 5 lines up (over the middle of the window).
//! `NOOB_THEME=light` shows the light look.
//!
//! The picture is a PPM (the simplest picture format there is, so no extra
//! crate is needed). On a Mac, `sips -s format png shot.ppm --out shot.png`
//! turns it into a PNG.

use std::io::Write;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use eframe::egui::{self, ColorImage, ScreenshotCallback, ViewportCommand};

const DEFAULT_WAIT: Duration = Duration::from_secs(4);
const KEYS_AFTER: Duration = Duration::from_millis(2500);

/// Adds the `NOOB_KEYS` presses to egui's input, one piece per frame (like a
/// person typing), so a shortcut can move the keyboard before the next piece.
pub fn press_keys(raw_input: &mut egui::RawInput) {
    static STARTED: OnceLock<Instant> = OnceLock::new();
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let Ok(keys) = std::env::var("NOOB_KEYS") else {
        return;
    };
    if STARTED.get_or_init(Instant::now).elapsed() < KEYS_AFTER {
        return;
    }
    let index = NEXT.fetch_add(1, Ordering::SeqCst);
    if let Some(piece) = keys.split('|').nth(index) {
        raw_input.events.extend(key_events(piece));
    }
}

fn key_events(piece: &str) -> Vec<egui::Event> {
    // A press and its release, like a real key.
    let press = |name: &str, modifiers: egui::Modifiers| {
        egui::Key::from_name(name)
            .map(|key| {
                [true, false].map(|pressed| egui::Event::Key {
                    key,
                    physical_key: None,
                    pressed,
                    repeat: false,
                    modifiers,
                })
            })
            .into_iter()
            .flatten()
    };
    match piece.split_once(':') {
        Some(("text", text)) => vec![egui::Event::Text(text.to_string())],
        Some(("key", name)) => press(name, egui::Modifiers::NONE).collect(),
        Some(("ctrl", name)) => press(name, egui::Modifiers::CTRL).collect(),
        Some(("cmd", name)) => press(name, egui::Modifiers::COMMAND).collect(),
        Some(("wheel", lines)) => vec![
            egui::Event::PointerMoved(egui::pos2(400.0, 300.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Line,
                delta: egui::vec2(0.0, lines.parse().unwrap_or(0.0)),
                phase: egui::TouchPhase::Move,
                source: egui::MouseWheelSource::Unknown,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        _ => Vec::new(),
    }
}

/// Call once per frame.
pub fn maybe_take(ctx: &egui::Context) {
    if std::env::var("NOOB_THEME").as_deref() == Ok("light") {
        ctx.set_theme(egui::Theme::Light);
    }
    static STARTED: OnceLock<Instant> = OnceLock::new();
    static ASKED: AtomicBool = AtomicBool::new(false);
    let Ok(path) = std::env::var("NOOB_SHOT_TO") else {
        return;
    };
    let wait = std::env::var("NOOB_SHOT_AFTER_MS")
        .ok()
        .and_then(|ms| ms.parse().ok())
        .map_or(DEFAULT_WAIT, Duration::from_millis);
    let started = STARTED.get_or_init(Instant::now);
    if started.elapsed() < wait {
        ctx.request_repaint_after(wait - started.elapsed());
        return;
    }
    if ASKED.swap(true, Ordering::SeqCst) {
        return;
    }
    let closer = ctx.clone();
    let callback = ScreenshotCallback::new(move |image| {
        if let Err(error) = save_ppm(&path, &image) {
            eprintln!("couldn't save the screenshot: {error}");
        }
        closer.send_viewport_cmd(ViewportCommand::Close);
        closer.request_repaint();
    });
    ctx.send_viewport_cmd(ViewportCommand::Screenshot(callback));
    ctx.request_repaint();
}

fn save_ppm(path: &str, image: &ColorImage) -> std::io::Result<()> {
    let [width, height] = image.size;
    let mut file = std::io::BufWriter::new(std::fs::File::create(path)?);
    write!(file, "P6\n{width} {height}\n255\n")?;
    for pixel in &image.pixels {
        file.write_all(&[pixel.r(), pixel.g(), pixel.b()])?;
    }
    file.flush()
}
