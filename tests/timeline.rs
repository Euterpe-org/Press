//! Driving the timeline for real, without a window.
//!
//! The trim handles are the one part of Press a charter manipulates directly,
//! and reading the paint code proves nothing about whether a drag lands. So
//! these feed pointer events into a real `egui::Context` and assert what came
//! out: genuine hit-testing, genuine drag thresholds, no desktop involved.
//!
//! Written after driving the actual window with synthetic mouse input, which
//! worked exactly once and then clicked on somebody's browser. A test that
//! needs the machine's real cursor is a test that borrows the machine.
//!
//! It earned itself immediately: the first grab test used the pointer's
//! position at `drag_started()`, which egui reports only after the movement
//! threshold — by then the pointer has left the handle, so a quick drag was
//! read as a seek and a slow one as a trim. Deterministic, and it presents as
//! flaky.

use eframe::egui::{self, Event, PointerButton, Pos2, RawInput, Rect, pos2, vec2};

use euterpe_press::plan::Trim;
use euterpe_press::timeline::{Change, timeline};

const WIDTH: f32 = 600.0;
const DURATION: f64 = 100.0;

struct Harness {
    ctx: egui::Context,
    trim: Trim,
    changes: Vec<Change>,
    rect: Rect,
}

impl Harness {
    fn new() -> Self {
        Self {
            ctx: egui::Context::default(),
            trim: Trim::default(),
            changes: Vec::new(),
            rect: Rect::NOTHING,
        }
    }

    /// One frame, with `events` delivered to it.
    fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(WIDTH, 200.0))),
            events,
            ..Default::default()
        };
        let trim = &mut self.trim;
        let changes = &mut self.changes;
        let rect = &mut self.rect;
        // The widget is given a rect the test chose, so both sides agree on
        // where the handles are to the pixel. Reading the geometry back out of
        // the Ui afterwards does not work: a root Ui's `min_rect` is the whole
        // screen, and computing it from `available_width` put the test's idea of
        // the end handle a few pixels from the widget's -- every press landing
        // just outside the grab radius, a test failing for its own arithmetic
        // while the code under it was right.
        *rect = Rect::from_min_size(pos2(20.0, 40.0), vec2(WIDTH - 40.0, 46.0));
        let target = *rect;
        let mut output = self.ctx.run_ui(input, |ui| {
            ui.scope_builder(egui::UiBuilder::new().max_rect(target), |ui| {
                changes.push(timeline(ui, DURATION, 0.0, trim, true));
            });
        });
        // There is no renderer here to upload the font atlas to, and epaint
        // panics on a dropped delta rather than leaking one. Discarding it is
        // correct for a test that never draws.
        output.textures_delta.clear();
    }

    fn settle(&mut self) {
        self.frame(vec![]);
        self.frame(vec![]);
    }

    /// x for a position in seconds, using the track the widget actually drew.
    fn x_at(&self, seconds: f64) -> f32 {
        let left = self.rect.left();
        let width = self.rect.width();
        left + (seconds / DURATION) as f32 * width
    }

    fn press(&mut self, x: f32) {
        let pos = pos2(x, self.rect.center().y);
        self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ]);
    }

    /// Moves in one jump, which is what a fast drag looks like and what broke
    /// the first implementation.
    fn drag_to(&mut self, x: f32) {
        let pos = pos2(x, self.rect.center().y);
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::PointerMoved(pos)]);
    }

    fn release(&mut self, x: f32) {
        let pos = pos2(x, self.rect.center().y);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }]);
    }

    fn trimmed(&self) -> bool {
        self.changes
            .iter()
            .any(|c| matches!(c, Change::Trim { .. }))
    }

    fn seeked(&self) -> bool {
        self.changes.iter().any(|c| matches!(c, Change::Seek(_)))
    }
}

/// Grabbing the END handle and pulling it left shortens the kept region. In one
/// jump, because that is the case the naive implementation got wrong.
#[test]
fn dragging_the_end_handle_shortens_what_is_kept() {
    let mut h = Harness::new();
    h.settle();
    assert!(h.trim.is_untouched(), "nothing is trimmed to begin with");

    let end = h.x_at(DURATION);
    h.changes.clear();
    h.press(end - 2.0);
    h.drag_to(h.x_at(60.0));
    h.release(h.x_at(60.0));

    assert!(
        h.trimmed(),
        "a drag on the handle must trim: {:?}",
        h.changes
    );
    assert!(
        !h.seeked(),
        "and must not ALSO be read as a seek: {:?}",
        h.changes
    );
    let kept = h.trim.keep_until.expect("the tail was cut");
    assert!(
        (kept - 60.0).abs() < 2.0,
        "the handle should land where it was dropped, got {kept}"
    );
    assert!(!h.trim.moves_the_opening(), "the opening was never touched");
}

/// Grabbing the START handle cuts the opening -- the act with consequences, and
/// the one the change reports separately so the window can say so.
#[test]
fn dragging_the_start_handle_reports_that_the_opening_moved() {
    let mut h = Harness::new();
    h.settle();

    let start = h.x_at(0.0);
    h.changes.clear();
    h.press(start + 2.0);
    h.drag_to(h.x_at(20.0));
    h.release(h.x_at(20.0));

    assert!(
        h.changes.iter().any(|c| matches!(
            c,
            Change::Trim {
                opening_moved: true
            }
        )),
        "the opening moving is reported as such: {:?}",
        h.changes
    );
    assert!(h.trim.moves_the_opening());
    assert!(
        (h.trim.cut_opening - 20.0).abs() < 2.0,
        "{}",
        h.trim.cut_opening
    );
    assert_eq!(h.trim.keep_until, None, "the tail was not touched");
}

/// Pressing in the middle of the track is a seek, not a trim. Somewhere to look,
/// not something to cut.
#[test]
fn pressing_away_from_the_handles_only_seeks() {
    let mut h = Harness::new();
    h.settle();

    h.changes.clear();
    let middle = h.x_at(50.0);
    h.press(middle);
    h.drag_to(h.x_at(55.0));
    h.release(h.x_at(55.0));

    assert!(h.seeked(), "the middle of the track seeks: {:?}", h.changes);
    assert!(!h.trimmed(), "and never trims: {:?}", h.changes);
    assert!(h.trim.is_untouched());
}

/// A handle dropped back against its own edge is not a trim of nearly-nothing,
/// it is no trim at all. Without the snap, a handle a pixel from the end forces
/// a whole re-encode for a cut nobody can see.
#[test]
fn a_handle_returned_to_the_edge_leaves_no_trim_behind() {
    let mut h = Harness::new();
    h.settle();

    let end = h.x_at(DURATION);
    h.press(end - 2.0);
    h.drag_to(h.x_at(40.0));
    h.release(h.x_at(40.0));
    assert!(h.trim.keep_until.is_some(), "trimmed first");

    // Now put it back.
    let handle_now = h.x_at(40.0);
    h.press(handle_now);
    h.drag_to(end);
    h.release(end);

    assert_eq!(h.trim.keep_until, None, "back at the edge means no trim");
    assert!(h.trim.is_untouched());
}

/// The two handles cannot be dragged through each other into a region of
/// negative length.
#[test]
fn the_region_cannot_be_turned_inside_out() {
    let mut h = Harness::new();
    h.settle();

    // Cut the tail down to 30s, then try to drag the opening past it.
    let end = h.x_at(DURATION);
    h.press(end - 2.0);
    h.drag_to(h.x_at(30.0));
    h.release(h.x_at(30.0));

    let start = h.x_at(0.0);
    h.press(start + 2.0);
    h.drag_to(h.x_at(90.0));
    h.release(h.x_at(90.0));

    let kept = h.trim.kept_seconds(DURATION);
    assert!(kept > 0.0, "the kept region still has a length: {kept}");
    assert!(
        h.trim.cut_opening < h.trim.keep_until.unwrap_or(DURATION),
        "start {} must stay before end {:?}",
        h.trim.cut_opening,
        h.trim.keep_until
    );
}
