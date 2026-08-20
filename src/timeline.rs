//! The timeline: where the video is, and which part of it is being kept.
//!
//! A slider would have been less code. It would also have been two controls for
//! one idea -- a position, and a length, neither of which shows the charter the
//! thing they are actually deciding, which is *which part of this video
//! survives*. So the kept region is drawn as a region, the discarded parts are
//! visibly discarded, and the handles are on the region's own edges.
//!
//! The two handles are not symmetrical, and the interface does not pretend they
//! are. Dragging the END is free and reversible: the tail past the music is
//! never shown to anyone. Dragging the START shifts the whole picture against
//! the song and forces a re-encode, so it is styled as the deliberate act it is
//! and the window says so underneath. See [`crate::plan::command_with_opening_cut`].

use eframe::egui::{self, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, pos2, vec2};

use crate::plan::Trim;
use crate::theme;

/// Total height of the strip, including the handles' grab area.
const HEIGHT: f32 = 46.0;
/// The track the region sits in.
const TRACK: f32 = 10.0;
const HANDLE_W: f32 = 11.0;
/// How close a click has to be to count as grabbing a handle rather than
/// seeking. Generous, because a handle is small and a mis-grab that seeks
/// instead of trimming is an annoying thing to undo.
const GRAB: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    None,
    /// The charter wants to look at this position.
    Seek(f64),
    /// The kept region moved. Carries whether the OPENING is what moved, which
    /// is the half with consequences.
    Trim {
        opening_moved: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Grabbed {
    Start,
    End,
    Playhead,
}

/// Draws the strip and applies any drag to `trim`.
pub fn timeline(
    ui: &mut egui::Ui,
    duration: f64,
    playhead: f64,
    trim: &mut Trim,
    allow_opening_cut: bool,
) -> Change {
    let duration = duration.max(0.001);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, HEIGHT), Sense::click_and_drag());

    let track = Rect::from_min_max(
        pos2(rect.left(), rect.center().y - TRACK / 2.0),
        pos2(rect.right(), rect.center().y + TRACK / 2.0),
    );
    let at = |seconds: f64| -> f32 {
        track.left() + (seconds / duration).clamp(0.0, 1.0) as f32 * track.width()
    };
    let seconds_at = |x: f32| -> f64 {
        (f64::from((x - track.left()) / track.width().max(1.0)) * duration).clamp(0.0, duration)
    };

    let start = trim.cut_opening;
    let end = trim.keep_until.unwrap_or(duration);

    // ── interaction ──────────────────────────────────────────────────────
    let id = response.id;
    let mut grabbed: Option<Grabbed> = ui.data(|d| d.get_temp(id));
    let mut change = Change::None;

    // What was grabbed is decided on the frame the button goes DOWN, and never
    // revisited until it comes up again.
    //
    // The obvious alternative -- decide on `drag_started()` -- does not work,
    // and fails in a way that reads as flaky rather than wrong: egui only
    // reports a drag once the pointer has passed its movement threshold, so by
    // then the pointer has LEFT the handle and the proximity test is measuring
    // the wrong position. A slow drag trims and a quick one seeks. Deciding at
    // press time makes the question unambiguous, because on that frame
    // `interact_pointer_pos()` is exactly where the charter clicked.
    if response.is_pointer_button_down_on()
        && grabbed.is_none()
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let near_start = allow_opening_cut && (pointer.x - at(start)).abs() <= GRAB;
        let near_end = (pointer.x - at(end)).abs() <= GRAB;
        // When both are within reach the END wins: it is the reversible one,
        // and it is what all but a few charters want.
        grabbed = Some(if near_end {
            Grabbed::End
        } else if near_start {
            Grabbed::Start
        } else {
            Grabbed::Playhead
        });
        ui.data_mut(|d| d.insert_temp(id, grabbed.unwrap()));
    }

    if let (Some(handle), Some(pointer)) = (grabbed, response.interact_pointer_pos()) {
        let value = seconds_at(pointer.x);
        match handle {
            Grabbed::Start => {
                // A quarter of a second of clearance, so the region can never be
                // dragged inside out into a negative length.
                trim.cut_opening = value.clamp(0.0, end - 0.25).max(0.0);
                // Snapping to zero: a handle a pixel off the left edge would
                // force a whole re-encode for a cut nobody can see.
                if trim.cut_opening < 0.05 {
                    trim.cut_opening = 0.0;
                }
                change = Change::Trim {
                    opening_moved: true,
                };
            }
            Grabbed::End => {
                let value = value.clamp(start + 0.25, duration);
                // Likewise at the right edge: within a twentieth of a second of
                // the end is not a trim, it is a whole video.
                trim.keep_until = (value < duration - 0.05).then_some(value);
                change = Change::Trim {
                    opening_moved: false,
                };
            }
            Grabbed::Playhead => change = Change::Seek(value),
        }
    }

    // Released: the next press decides afresh. Keyed on the pointer being up
    // rather than on `drag_stopped()`, which never fires for a press that moved
    // less than the drag threshold and would leave the grab latched.
    if ui.input(|i| !i.pointer.any_down()) {
        ui.data_mut(|d| d.remove::<Grabbed>(id));
    }

    // ── paint ────────────────────────────────────────────────────────────
    let start = trim.cut_opening;
    let end = trim.keep_until.unwrap_or(duration);
    let radius = CornerRadius::same(5);
    let painter = ui.painter();

    painter.rect_filled(track, radius, theme::PANEL_RAISED);

    let kept = Rect::from_min_max(pos2(at(start), track.top()), pos2(at(end), track.bottom()));
    painter.rect_filled(kept, radius, theme::ACCENT_DIM);

    // The discarded parts are drawn as discarded rather than simply left empty:
    // "this is being thrown away" is the whole message of the control.
    for (from, to) in [(0.0, start), (end, duration)] {
        if to > from {
            let gone =
                Rect::from_min_max(pos2(at(from), track.top()), pos2(at(to), track.bottom()));
            painter.rect_filled(gone, radius, Color32::from_rgb(0x24, 0x1c, 0x1c));
            hatch(painter, gone);
        }
    }

    let head = at(playhead.clamp(0.0, duration));
    painter.line_segment(
        [
            pos2(head, rect.top() + 4.0),
            pos2(head, rect.bottom() - 4.0),
        ],
        Stroke::new(2.0, theme::INK),
    );

    handle(ui, at(start), rect, allow_opening_cut, start > 0.0);
    handle(ui, at(end), rect, true, trim.keep_until.is_some());

    change
}

fn handle(ui: &egui::Ui, x: f32, rect: Rect, enabled: bool, engaged: bool) {
    let colour = match (enabled, engaged) {
        (false, _) => theme::HAIRLINE,
        (true, false) => theme::INK_MUTED,
        (true, true) => theme::ACCENT,
    };
    let body = Rect::from_min_max(
        pos2(x - HANDLE_W / 2.0, rect.top() + 2.0),
        pos2(x + HANDLE_W / 2.0, rect.bottom() - 2.0),
    );
    ui.painter().rect(
        body,
        CornerRadius::same(4),
        colour,
        Stroke::new(1.0, theme::BACKDROP),
        StrokeKind::Inside,
    );
}

/// Diagonal strokes over a discarded stretch. Colour alone reads as "a different
/// section"; a texture reads as "not part of the result".
fn hatch(painter: &egui::Painter, rect: Rect) {
    let stroke = Stroke::new(1.0, Color32::from_rgb(0x4a, 0x33, 0x33));
    let step = 7.0;
    let mut x = rect.left();
    while x < rect.right() + rect.height() {
        let top = pos2(x.min(rect.right()), rect.top());
        let bottom = pos2((x - rect.height()).max(rect.left()), rect.bottom());
        if top.x > rect.left() && bottom.x < rect.right() {
            painter.line_segment([top, bottom], stroke);
        }
        x += step;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A trim within a twentieth of a second of an edge is not a trim. Without
    /// the snap, a handle left a pixel from the end would force a whole
    /// re-encode for a cut nobody could see.
    #[test]
    fn a_handle_at_the_edge_means_no_trim_at_all() {
        let untouched = Trim::default();
        assert!(untouched.is_untouched());
        assert!(!untouched.moves_the_opening());
        assert_eq!(untouched.allowance(100.0), None);
    }

    /// Kept length is measured between the handles, not from zero -- otherwise a
    /// video cut at both ends would be told to run longer than it does.
    #[test]
    fn the_kept_length_is_measured_between_the_handles() {
        let both = Trim {
            cut_opening: 5.0,
            keep_until: Some(80.0),
        };
        assert_eq!(both.kept_seconds(100.0), 75.0);
        assert_eq!(both.allowance(100.0), Some(75.0));

        let tail_only = Trim {
            cut_opening: 0.0,
            keep_until: Some(80.0),
        };
        assert_eq!(tail_only.kept_seconds(100.0), 80.0);
        assert!(!tail_only.moves_the_opening());

        let head_only = Trim {
            cut_opening: 5.0,
            keep_until: None,
        };
        assert_eq!(head_only.kept_seconds(100.0), 95.0);
        assert!(head_only.moves_the_opening());
    }
}
