//! How the window looks.
//!
//! egui's defaults are a debug skin -- grey on grey, everything the same size,
//! every corner the same radius. They are fine for a tool that inspects a game
//! engine and wrong for one a charter opens between exporting a video and
//! uploading it.
//!
//! The whole surface is one drop zone, one verdict and one button, so the design
//! has exactly one job: make the verdict unmissable and everything else quiet.
//! Hence one accent colour, used only for the thing to press; findings in amber
//! because they are not errors -- Press is about to fix them -- and a real
//! typographic scale so the answer is legible from across a desk.

use eframe::egui::{self, Color32, CornerRadius, Stroke};

/// Euterpe's blue, the same one the site's badges use.
pub const ACCENT: Color32 = Color32::from_rgb(0x0e, 0xa5, 0xe9);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x07, 0x6c, 0x99);

/// Findings are amber, never red. Red says "you broke something"; amber says
/// "noted, and it is being handled", which is what is actually true here.
pub const NOTICE: Color32 = Color32::from_rgb(0xf5, 0xa5, 0x24);

/// Reserved for the one case Press cannot fix: an unreadable file.
pub const ALARM: Color32 = Color32::from_rgb(0xef, 0x53, 0x50);

pub const GOOD: Color32 = Color32::from_rgb(0x34, 0xd3, 0x99);

pub const BACKDROP: Color32 = Color32::from_rgb(0x10, 0x14, 0x1a);
pub const PANEL: Color32 = Color32::from_rgb(0x18, 0x1e, 0x27);
pub const PANEL_RAISED: Color32 = Color32::from_rgb(0x21, 0x29, 0x34);
pub const HAIRLINE: Color32 = Color32::from_rgb(0x2c, 0x35, 0x42);

pub const INK: Color32 = Color32::from_rgb(0xe8, 0xed, 0xf4);
pub const INK_MUTED: Color32 = Color32::from_rgb(0x8c, 0x99, 0xab);

pub const RADIUS: u8 = 10;

/// Named sizes rather than numbers at each call site, so the scale stays a scale.
pub mod size {
    pub const HERO: f32 = 26.0;
    pub const TITLE: f32 = 18.0;
    pub const BODY: f32 = 14.5;
    pub const SMALL: f32 = 12.5;
}

/// Four gaps, and nothing between them.
///
/// The first layout used 4, 6, 8, 10, 12 and 14 wherever each looked right on
/// its own, which is how a window ends up with slack in one place and things
/// touching in another: every gap was a local decision and none of them agreed.
/// A gap now means something -- inside a thought, between thoughts, between
/// blocks, between regions -- and there is no fifth meaning available.
pub mod space {
    /// Inside one idea: a line and its explanation.
    pub const TIGHT: f32 = 6.0;
    /// Between rows of the same thing.
    pub const ROW: f32 = 12.0;
    /// Between one block and the next.
    pub const BLOCK: f32 = 20.0;
    /// Between regions of the window.
    pub const SECTION: f32 = 28.0;
}

/// A hairline across the full width, for separating regions.
pub fn rule(ui: &mut egui::Ui) {
    let y = ui.cursor().top();
    ui.painter()
        .hline(ui.max_rect().x_range(), y, Stroke::new(1.0, HAIRLINE));
    ui.add_space(1.0);
}

pub fn install(ctx: &egui::Context) {
    ctx.all_styles_mut(dress);
}

fn dress(style: &mut egui::Style) {
    use egui::FontFamily::Proportional;
    use egui::TextStyle::{Body, Button, Heading, Monospace, Small};
    use egui::{FontId, Visuals};

    style.text_styles = [
        (Heading, FontId::new(size::HERO, Proportional)),
        (Body, FontId::new(size::BODY, Proportional)),
        (Button, FontId::new(size::BODY, Proportional)),
        (Small, FontId::new(size::SMALL, Proportional)),
        (
            Monospace,
            FontId::new(size::SMALL, egui::FontFamily::Monospace),
        ),
    ]
    .into();

    let mut visuals = Visuals::dark();
    visuals.panel_fill = BACKDROP;
    visuals.window_fill = PANEL;
    visuals.extreme_bg_color = BACKDROP;
    visuals.override_text_color = Some(INK);
    visuals.hyperlink_color = ACCENT;
    visuals.selection.bg_fill = ACCENT_DIM;
    visuals.selection.stroke = Stroke::new(1.0, INK);

    let radius = CornerRadius::same(RADIUS);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = radius;
    }
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, HAIRLINE);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, INK_MUTED);

    visuals.widgets.inactive.bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.weak_bg_fill = PANEL_RAISED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, HAIRLINE);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, INK);

    visuals.widgets.hovered.bg_fill = HAIRLINE;
    visuals.widgets.hovered.weak_bg_fill = HAIRLINE;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT_DIM);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, INK);

    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.weak_bg_fill = ACCENT_DIM;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, INK);

    style.visuals = visuals;
    // VERTICAL SPACING IS ZERO, and that is the point: egui adds `item_spacing`
    // after every widget, so a written `add_space(TIGHT)` was really 6 + 10 = 16
    // and every gap in the window was a number nobody chose. With it at zero the
    // four gaps in `space` are the only vertical gaps that exist.
    //
    // Horizontally it stays, because a row of controls does want air between its
    // items and there is no cursor to place them by.
    style.spacing.item_spacing = egui::vec2(space::TIGHT, 0.0);
    style.spacing.button_padding = egui::vec2(16.0, 9.0);
    style.spacing.window_margin = egui::Margin::same(0);
}

/// Padding inside a card, one value so a caller can subtract it.
pub const CARD_PAD_X: i8 = 18;
pub const CARD_PAD_Y: i8 = 16;

fn panel() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, HAIRLINE))
        .corner_radius(CornerRadius::same(RADIUS + 2))
        .inner_margin(egui::Margin::symmetric(CARD_PAD_X, CARD_PAD_Y))
}

/// A panel with a hairline border, used for every grouped block so the window
/// reads as a few surfaces rather than a list of controls.
pub fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    panel().show(ui, add).response
}

/// A card of an exact outer height, for filling a column on a fixed canvas.
///
/// The height is given rather than grown into, because a card that asks its
/// parent to be taller is how a 960x640 window ended up laying out 1084px of
/// content.
pub fn card_of_height(
    ui: &mut egui::Ui,
    outer: f32,
    add: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let width = ui.available_width();
    let rect = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(width, outer));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    child.set_clip_rect(rect);
    let response = panel().show(&mut child, |ui| {
        ui.set_min_height(outer - 2.0 * f32::from(CARD_PAD_Y));
        add(ui);
    });
    ui.advance_cursor_after_rect(rect);
    response.response
}

/// The height EVERY button in an action row shares.
///
/// One height, because two buttons side by side at two heights read as a
/// mistake rather than as a hierarchy -- weight is carried by fill and by type
/// size, which is what actually says "this one first".
pub const BUTTON_H: f32 = 44.0;

/// Ink on the accent. Dark rather than white: the accent is bright enough that
/// white on it is the lower-contrast pair.
const ON_ACCENT: Color32 = Color32::from_rgb(6, 12, 18);

/// The accent one step brighter, for hover, and one step darker for the press.
const ACCENT_HOT: Color32 = Color32::from_rgb(0x38, 0xbd, 0xf8);

enum Weight {
    Loud,
    Quiet,
}

/// Hand-painted rather than an `egui::Button`, for one reason: `Button::fill`
/// overrides the fill in EVERY state, so a filled button has no hover at all.
/// The accent one is the thing the whole window is pointing at, and it was the
/// only control on screen that did not react to the pointer.
fn button(ui: &mut egui::Ui, label: &str, weight: Weight) -> egui::Response {
    let (font, pad_x) = match weight {
        Weight::Loud => (egui::FontId::proportional(size::TITLE), 26.0),
        Weight::Quiet => (egui::FontId::proportional(size::BODY), 20.0),
    };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_owned(), font, Color32::PLACEHOLDER);
    let measured = galley.size();
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(measured.x + pad_x * 2.0, BUTTON_H),
        egui::Sense::click(),
    );

    let down = response.is_pointer_button_down_on();
    let hot = response.hovered();
    let (fill, edge, ink) = match (weight, down, hot) {
        (Weight::Loud, true, _) => (ACCENT_DIM, ACCENT_DIM, ON_ACCENT),
        // A bright rim in the text colour, not a barely-lighter fill: on a
        // saturated blue a shade change is hard to see at all.
        (Weight::Loud, false, true) => (ACCENT_HOT, INK, ON_ACCENT),
        (Weight::Loud, false, false) => (ACCENT, ACCENT, ON_ACCENT),
        (Weight::Quiet, true, _) => (BACKDROP, HAIRLINE, INK_MUTED),
        (Weight::Quiet, false, true) => (PANEL_RAISED, ACCENT, INK),
        (Weight::Quiet, false, false) => (PANEL, HAIRLINE, INK_MUTED),
    };

    let painter = ui.painter();
    painter.rect(
        rect,
        CornerRadius::same(RADIUS),
        fill,
        Stroke::new(1.5, edge),
        egui::StrokeKind::Inside,
    );
    painter.galley(rect.center() - measured / 2.0, galley, ink);

    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The one loud control on the screen. There is never more than one.
pub fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    button(ui, label, Weight::Loud)
}

/// A transport control. **Exactly** 38x30: the padding is zeroed so the size is
/// the size, which means play and pause -- two glyphs of different widths -- do
/// not resize the row they sit in and shove the clock sideways every time
/// playback starts.
///
/// The glyph is TITLE-sized, the same as the clock beside it. A chunky button
/// around a small glyph next to a number of a third size was the row reading as
/// three unrelated things; at one size they read as one control.
pub fn transport_button(ui: &mut egui::Ui, glyph: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::Vec2::ZERO;
        ui.add(
            egui::Button::new(egui::RichText::new(glyph).size(size::TITLE).color(INK))
                .fill(PANEL_RAISED)
                .corner_radius(CornerRadius::same(RADIUS - 2))
                .min_size(egui::vec2(38.0, 30.0)),
        )
    })
    .inner
}

/// A small outlined button for use INSIDE a control row, where `quiet` -- sized
/// for the action bar -- would out-weigh the control it sits beside.
pub fn chip(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(10.0, 5.0);
        ui.add(
            egui::Button::new(
                egui::RichText::new(label)
                    .size(size::SMALL)
                    .color(INK_MUTED),
            )
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::new(1.0, HAIRLINE))
            .corner_radius(CornerRadius::same(RADIUS - 4)),
        )
    })
    .inner
}

/// The scrubber's readout. The position is the number that is changing and the
/// duration is the context it changes within, so they are not the same size:
/// one is the value, the other is its scale.
pub fn clock_pair(ui: &mut egui::Ui, now: &str, total: &str) {
    ui.label(
        egui::RichText::new(now)
            .monospace()
            .size(size::TITLE)
            .color(INK),
    );
    ui.label(
        egui::RichText::new(format!("/ {total}"))
            .monospace()
            .size(size::SMALL)
            .color(INK_MUTED),
    );
}

/// The secondary control in an action row. Same HEIGHT as `primary`, lower
/// weight -- two buttons side by side at two different heights read as a
/// mistake, not as a hierarchy.
pub fn quiet(ui: &mut egui::Ui, label: &str) -> egui::Response {
    button(ui, label, Weight::Quiet)
}

pub fn heading(ui: &mut egui::Ui, text: &str, colour: Color32) {
    ui.label(
        egui::RichText::new(text)
            .size(size::TITLE)
            .strong()
            .color(colour),
    );
}

pub fn muted(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(size::SMALL).color(INK_MUTED));
}

/// A file path, in full.
///
/// Monospaced and wrapping, because a path is read by SCANNING it rather than
/// by reading it as prose: separators and repeated folder names line up, and the
/// part that differs between two similar paths is what the eye is looking for.
/// Selectable, so it can be copied out.
pub fn path(ui: &mut egui::Ui, full: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(full)
                .monospace()
                .size(size::SMALL)
                .color(INK_MUTED),
        )
        .wrap(),
    );
}
