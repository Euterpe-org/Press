//! Placing things on a canvas whose size is known in advance.
//!
//! The window is FIXED, and everything here follows from that. There is no
//! negotiation, no scrolling, no reflow -- regions are cut out of a rectangle
//! measured once, and content that does not fit is a bug rather than a condition
//! to handle at runtime. [`region`] is where that is enforced: it asserts, by
//! name, that what a screen laid out fits the space it was handed.
//!
//! Kept apart from the screens because it is about PLACEMENT and they are about
//! content -- and because these are the pieces whose failures are silent. A bar
//! two pixels short of its band, or a row that advanced the cursor by the space
//! it was offered rather than the space it used, hides overflow from the very
//! assertion written to catch it. Both happened.

use eframe::egui;

use super::*;

pub(super) fn slice_top(rect: egui::Rect, height: f32) -> egui::Rect {
    egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + height))
}

pub(super) fn slice_bottom(rect: egui::Rect, height: f32) -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - height), rect.max)
}

pub(super) fn hairline(ui: &egui::Ui, x: egui::Rangef, y: f32) {
    ui.painter()
        .hline(x, y, egui::Stroke::new(1.0, theme::HAIRLINE));
}

/// Draws into exactly `rect` and nowhere else.
///
/// The clip makes overflow impossible to SEE, which is only half of what is
/// wanted: content that does not fit is then silently cut instead of spilling,
/// and a cut sentence is a worse bug than a visible one because nobody notices
/// it. So a debug build also asserts that what was laid out actually fitted --
/// every test run is a check that the fixed canvas is big enough for what is on
/// it, in every language.
pub(super) fn region(
    ui: &mut egui::Ui,
    name: &str,
    rect: egui::Rect,
    add: impl FnOnce(&mut egui::Ui),
) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(rect);
    add(&mut child);

    // Named, because "a region overflowed" is a fact nobody can act on. There
    // are eight of them and they nest, so the first question on reading the
    // failure is always which one -- and answering it by bisecting the test is
    // several minutes that this string costs nothing to save.
    let used = child.min_rect();
    debug_assert!(
        used.height() <= rect.height() + 1.0 && used.width() <= rect.width() + 1.0,
        "{name} laid out {:.0}x{:.0} into {:.0}x{:.0} and the excess was clipped away",
        used.width(),
        used.height(),
        rect.width(),
        rect.height(),
    );
}

/// A sentence painted centred and wrapped, for the one place with no widget to
/// hang it off -- the drop zone is a single painted rectangle by design.
pub(super) fn wrapped_centre(
    ui: &egui::Ui,
    centre: egui::Pos2,
    body: &str,
    colour: egui::Color32,
    width: f32,
) {
    let mut job = egui::text::LayoutJob::simple(
        body.to_owned(),
        egui::FontId::proportional(theme::size::BODY),
        colour,
        width,
    );
    job.halign = egui::Align::Center;
    let galley = ui.painter().layout_job(job);
    let top = centre - egui::vec2(0.0, galley.size().y / 2.0);
    ui.painter().galley(top, galley, colour);
}

/// How wide a centred block is allowed to be.
///
/// 520 in a 960 window was a narrow column with a lot of air either side, and
/// the narrowness cost lines: a finding worded in German wrapped to three where
/// it fits in two. 640 keeps prose near a comfortable measure and stops the
/// window looking like it is using a third of itself.
pub(super) const CENTRED_W: f32 = 640.0;

/// Centres a block in whatever is left, at whatever height it turns out to be.
///
/// The height is not guessed: it is measured after drawing and remembered, so
/// the next frame centres the real thing. A guessed height is a card that is
/// either clipped or floating slightly off-centre, and neither is visible enough
/// to notice or invisible enough to ignore. The first frame after a change is
/// off by the difference and the repaint timer settles it before an eye could
/// follow.
pub(super) fn centred(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    let key = egui::Id::new(id);
    let known: Option<f32> = ui.data(|d| d.get_temp(key));
    let room = ui.available_height();

    // Bounded by a scroll area, and the design is expected never to need it.
    //
    // Two different jobs. The debug assertion below is the one that matters: a
    // screen that has to scroll on a canvas built for it is a design that did
    // not fit, and it should fail a test. But when it does happen in the field
    // -- a language nobody measured, a filename nobody imagined -- content that
    // can be scrolled to is recoverable and content that was clipped away is
    // simply gone.
    let out = egui::ScrollArea::vertical()
        .id_salt(key)
        .max_height(room)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // NO SLACK UNTIL THE HEIGHT IS KNOWN. The first frame draws
            // top-aligned and measures; every frame after centres what was
            // measured. Guessing instead -- 140 was the guess -- pads by half
            // the difference between the guess and the truth, and a card that
            // turned out to be 368 tall in a body of 456 got 158 above it and
            // spilled off the bottom.
            let slack = known.map_or(0.0, |h| ((room - h) / 2.0).max(0.0));
            ui.add_space(slack);

            let width = ui.available_width().min(CENTRED_W);
            let mut measured = 0.0;
            ui.vertical_centered(|ui| {
                let start = ui.cursor().top();
                ui.allocate_ui(egui::vec2(width, (room - slack).max(1.0)), add);
                measured = ui.cursor().top() - start;
            });
            measured
        });

    let measured = out.inner;
    if known.is_none_or(|h| (measured - h).abs() > 0.5) {
        ui.data_mut(|d| d.insert_temp(key, measured));
        ui.ctx().request_repaint();
    }
    debug_assert!(
        measured <= room + 1.0,
        "{id} needs {measured:.0}px in {room:.0}px, so it will scroll -- it was meant to fit"
    );
}

/// A horizontal row of controls, centred on the width it actually takes.
///
/// egui has no way to do this directly: `vertical_centered` centres a single
/// widget, but a `horizontal` layout placed inside it claims the whole width and
/// lays its contents out from the left edge, so the row ends up in the corner.
/// So the width is measured after drawing and remembered, the same trick and for
/// the same reason as [`centred`] on the other axis.
pub(super) fn centred_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    let id = egui::Id::new(id);
    let known: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);

    let full = ui.max_rect();
    // Before it has ever been measured the row gets the whole width, so the
    // single frame that exists in that state is merely left-aligned rather than
    // hanging off one side.
    let width = if known > 0.0 {
        known.min(full.width())
    } else {
        full.width()
    };
    let band = egui::Rect::from_min_size(
        egui::pos2(full.center().x - width / 2.0, full.top()),
        egui::vec2(width, full.height()),
    );

    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(band));
    child.horizontal(|ui| add(ui));
    let used = child.min_rect();

    // Advanced by what the row ACTUALLY took, not by the band it was offered.
    // Advancing by the band would report a row that fitted whatever its
    // contents did, which hides exactly the overflow `region`'s assertion
    // exists to catch -- and did hide it: a bar two pixels too short for its
    // own buttons passed a test written to find that.
    ui.advance_cursor_after_rect(used);

    let measured = used.width();

    if (measured - known).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, measured));
        ui.ctx().request_repaint();
    }
}
