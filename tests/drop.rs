//! One drop is one load.
//!
//! Written to answer a question a screenshot asked: dropping a video made
//! console windows flash open and shut, over and over, indefinitely. The only
//! thing Press starts is ffmpeg, and the only ffmpeg it starts on a drop is the
//! viewfinder's — one per seek. So either the viewfinder was looping, or the
//! FILE was being loaded again every frame, each load starting a decoder and
//! dropping the previous one (which kills its process — hence open, shut, open,
//! shut).
//!
//! Reading the code did not settle it, so this measures it: a real
//! `egui::Context`, a real dropped-file handle, and a count of how many times
//! the window decided a file had arrived.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, RawInput, Rect, pos2};
use euterpe_press::app::Press;
use euterpe_press::i18n::Lang;

/// The integration owns the concrete handle in real life; this is the same
/// thing with a path in it.
#[derive(Debug)]
struct Dropped(PathBuf);

impl egui::DroppedFile for Dropped {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        std::fs::read(&self.0).map_err(|e| e.to_string())
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn blank_input() -> RawInput {
    sized_input(egui::Vec2::from(euterpe_press::app::WINDOW))
}

fn sized_input(size: egui::Vec2) -> RawInput {
    RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), size)),
        ..Default::default()
    }
}

/// Runs one frame of the real window and hands back what it painted.
fn frame(
    ctx: &egui::Context,
    press: &mut Press,
    input: RawInput,
) -> Vec<eframe::egui::epaint::ClippedShape> {
    let mut frame = eframe::Frame::_new_kittest();
    let mut output = ctx.run_ui(input, |ui| {
        use eframe::App;
        press.ui(ui, &mut frame);
    });
    // No renderer here to upload the font atlas to, and epaint panics on a
    // dropped delta rather than leaking one.
    output.textures_delta.clear();
    std::mem::take(&mut output.shapes)
}

/// The window is built with no encoder, so nothing is actually spawned: what is
/// being measured is the DECISION to load, not its consequences.
#[test]
fn one_drop_loads_the_file_exactly_once() {
    let ctx = egui::Context::default();
    let mut press = Press::new(&ctx, Lang::En, None);

    let _ = frame(&ctx, &mut press, blank_input());
    assert_eq!(press.loads_so_far(), 0, "nothing dropped yet");

    let mut dropped = blank_input();
    dropped
        .dropped_files
        .push(Arc::new(Dropped(fixture("plain_h264.mp4"))));
    let _ = frame(&ctx, &mut press, dropped);

    assert_eq!(press.loads_so_far(), 1, "the drop should load it once");

    // Sixty more frames with nothing in them -- a second of a window simply
    // being open. Every one of these that loads again is a decoder started and
    // killed, which is what the flashing consoles were.
    for _ in 0..60 {
        let _ = frame(&ctx, &mut press, blank_input());
    }
    assert_eq!(
        press.loads_so_far(),
        1,
        "the file was re-loaded on later frames — one drop must be one load"
    );
}

/// Content never spills out of the window.
///
/// It did. A portrait 1080x816 source drew a preview tall enough to push the
/// verdict card and the primary button off the bottom, and there was no
/// scrollbar to reach them with -- the one thing on screen to press was simply
/// not on screen. The fix reserves the action first and scrolls only what is
/// above it, and this is what stops that from coming back.
///
/// ONE size is checked, because there is only one: the window is fixed. That is
/// the whole reason the layout can be laid out rather than negotiated, and a
/// test sweeping sizes that cannot occur would be testing dead shapes.
#[test]
fn nothing_is_ever_drawn_outside_the_window() {
    let size = egui::Vec2::from(euterpe_press::app::WINDOW);
    let ctx = egui::Context::default();
    let mut press = Press::new(&ctx, Lang::En, None);

    // Two findings and a CJK filename: the tallest a readout gets.
    let mut input = sized_input(size);
    input
        .dropped_files
        .push(Arc::new(Dropped(fixture("audio_pixels_over.mp4"))));
    let _ = frame(&ctx, &mut press, input);
    // Settle: egui needs a pass to know how tall things are.
    let mut shapes = Vec::new();
    for _ in 0..3 {
        shapes = frame(&ctx, &mut press, sized_input(size));
    }

    // Measured on the SHAPES that were actually emitted, not on a layout
    // rectangle: the regions are clipped, so a layout figure would say the
    // content fits even where it had been cut off to make it fit. `region`
    // asserts the fitting; this asserts that nothing is painted outside the
    // glass.
    let screen = Rect::from_min_size(pos2(0.0, 0.0), size);
    let mut painted = Rect::NOTHING;
    for shape in &shapes {
        let bounds = shape
            .shape
            .visual_bounding_rect()
            .intersect(shape.clip_rect);
        if bounds.is_positive() {
            painted = painted.union(bounds);
        }
    }
    assert!(painted.is_positive(), "nothing was painted at all");
    assert!(
        screen.contains_rect(painted.shrink(0.5)),
        "painted {painted:?} outside a window of {screen:?}"
    );
}

/// Dropping a second file replaces the first, once.
#[test]
fn a_second_drop_replaces_the_first() {
    let ctx = egui::Context::default();
    let mut press = Press::new(&ctx, Lang::En, None);

    for name in ["plain_h264.mp4", "hevc.mp4"] {
        let mut input = blank_input();
        input.dropped_files.push(Arc::new(Dropped(fixture(name))));
        let _ = frame(&ctx, &mut press, input);
        for _ in 0..10 {
            let _ = frame(&ctx, &mut press, blank_input());
        }
    }

    assert_eq!(press.loads_so_far(), 2, "two drops, two loads");
}

/// The same file dropped again is a deliberate act -- reloading it is what the
/// charter asked for -- and must still be exactly one load.
#[test]
fn dropping_the_same_file_again_reloads_it_once() {
    let ctx = egui::Context::default();
    let mut press = Press::new(&ctx, Lang::En, None);

    for _ in 0..2 {
        let mut input = blank_input();
        input
            .dropped_files
            .push(Arc::new(Dropped(fixture("plain_h264.mp4"))));
        let _ = frame(&ctx, &mut press, input);
        for _ in 0..10 {
            let _ = frame(&ctx, &mut press, blank_input());
        }
    }

    assert_eq!(press.loads_so_far(), 2, "two gestures, two loads");
}
