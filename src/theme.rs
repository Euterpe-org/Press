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
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(16.0, 9.0);
    style.spacing.window_margin = egui::Margin::same(0);
}

/// A panel with a hairline border, used for every grouped block so the window
/// reads as a few surfaces rather than a list of controls.
pub fn card(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0, HAIRLINE))
        .corner_radius(CornerRadius::same(RADIUS + 2))
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, add)
        .response
}

/// The one loud control on the screen. There is never more than one.
pub fn primary(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let text = egui::RichText::new(label)
        .size(size::TITLE)
        .color(Color32::from_rgb(6, 12, 18));
    ui.add(
        egui::Button::new(text)
            .fill(ACCENT)
            .corner_radius(CornerRadius::same(RADIUS))
            .min_size(egui::vec2(0.0, 44.0)),
    )
}

pub fn quiet(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add(egui::Button::new(
        egui::RichText::new(label).color(INK_MUTED),
    ))
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
