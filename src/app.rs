//! The window.
//!
//! One file at a time, and one thing to press. A charter arrives having just
//! exported a video and wanting to know whether it will upload; everything here
//! serves that sentence and then gets out of the way.
//!
//! Four stages, and the screen only ever shows one of them: nothing loaded, a
//! file with a verdict, a run in progress, a result. There is no settings
//! screen, because there are no settings -- resolution, frame rate, codec and
//! size have one correct answer each and Press knows all four.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use eframe::egui;

use crate::ffmpeg::{Cancel, Ffmpeg};
use crate::gate::{self, Facts, Verdict, limits};
use crate::i18n::{Lang, Text, text};
use crate::plan::Trim;
use crate::preview::Preview;
use crate::standardise::{self, Outcome, Produced, Step, Treatment};
use crate::timeline::Change;
use crate::{fonts, theme};

pub struct Press {
    lang: Lang,
    ffmpeg: Option<Ffmpeg>,
    stage: Stage,
    texture: Option<egui::TextureHandle>,
    shown_pts: f64,
    /// A file chosen through the picker, waiting to be taken by the same code
    /// path a drop takes. One way in rather than two.
    pending: Option<PathBuf>,
}

enum Stage {
    Empty,
    Loaded(Box<Loaded>),
    Working(Box<Working>),
    Finished(Box<Finished>),
}

struct Loaded {
    path: PathBuf,
    source_bytes: u64,
    facts: Option<Facts>,
    verdict: Verdict,
    preview: Option<Preview>,
    /// Where the scrubber is, in seconds.
    at: f64,
    /// Which part of the source survives. Untouched by default: the opening is
    /// never moved unless the charter moves it.
    trim: Trim,
}

struct Working {
    path: PathBuf,
    source_bytes: u64,
    cancel: Cancel,
    inbox: Receiver<Note>,
    step: Step,
    /// A forced re-encode has no compliant copy behind it, so cancelling leaves
    /// nothing. The button has to say so.
    forced: bool,
}

struct Finished {
    source_bytes: u64,
    outcome: Outcome,
}

enum Note {
    At(Step),
    Done(Box<Outcome>),
}

impl Press {
    pub fn new(ctx: &egui::Context, lang: Lang, ffmpeg: Option<Ffmpeg>) -> Self {
        fonts::install(ctx, lang);
        theme::install(ctx);
        Self {
            lang,
            ffmpeg,
            stage: Stage::Empty,
            texture: None,
            shown_pts: f64::NAN,
            pending: None,
        }
    }

    /// Load a file the window was started with -- dropped on the exe, or opened
    /// "with" Press. Queued rather than read here so it takes the same path in
    /// as a drop, on the first frame, once there is a context to draw into.
    pub fn open(&mut self, path: &Path) {
        self.pending = Some(path.to_path_buf());
    }

    fn t(&self, text: &'static Text) -> &'static str {
        text.get(self.lang)
    }
}

impl eframe::App for Press {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.absorb_drops(&ctx);
        self.pump(&ctx);

        egui::Frame::new()
            .fill(theme::BACKDROP)
            .inner_margin(egui::Margin::same(22))
            .show(ui, |ui| {
                ui.set_min_size(ui.available_size());
                self.header(ui);
                ui.add_space(14.0);
                match &mut self.stage {
                    Stage::Empty => {
                        if let Some(picked) = empty(ui, self.lang, &ctx) {
                            self.pending = Some(picked);
                        }
                    }
                    Stage::Loaded(_) => self.loaded(ui, &ctx),
                    Stage::Working(_) => self.working(ui),
                    Stage::Finished(_) => self.finished(ui),
                }
            });
    }
}

impl Press {
    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(self.t(&text::WINDOW_TITLE))
                    .size(theme::size::TITLE)
                    .strong()
                    .color(theme::INK),
            );
            ui.label(
                egui::RichText::new(self.t(&text::TAGLINE))
                    .size(theme::size::SMALL)
                    .color(theme::INK_MUTED),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let current = self.lang;
                egui::ComboBox::from_id_salt("lang")
                    .selected_text(current.endonym())
                    .show_ui(ui, |ui| {
                        for lang in Lang::ALL {
                            if ui
                                .selectable_label(lang == current, lang.endonym())
                                .clicked()
                                && lang != current
                            {
                                self.lang = lang;
                                fonts::install(ui.ctx(), lang);
                            }
                        }
                    });
            });
        });
        ui.add_space(6.0);
        ui.painter().hline(
            ui.max_rect().x_range(),
            ui.cursor().top(),
            egui::Stroke::new(1.0, theme::HAIRLINE),
        );
    }

    /// A file dropped anywhere on the window replaces whatever is loaded --
    /// except while a run is going, where it would be ambiguous whether the drop
    /// meant "and this one too" or "stop and do this instead".
    fn absorb_drops(&mut self, ctx: &egui::Context) {
        if matches!(self.stage, Stage::Working(_)) {
            self.pending = None;
            return;
        }
        let dropped: Option<PathBuf> =
            ctx.input(|i| i.raw.dropped_files.first().map(|f| f.path().to_path_buf()));
        if let Some(path) = dropped.or_else(|| self.pending.take()) {
            self.load(&path);
        }
    }

    fn load(&mut self, path: &Path) {
        self.texture = None;
        self.shown_pts = f64::NAN;

        let Ok(bytes) = std::fs::read(path) else {
            self.stage = Stage::Empty;
            return;
        };
        let source_bytes = bytes.len() as u64;
        let (facts, verdict) = gate::judge(&bytes);
        drop(bytes);

        let preview = match (&facts, &self.ffmpeg) {
            (Some(f), Some(ffmpeg)) => {
                let preview = Preview::open(ffmpeg.binary(), path, f.width, f.height);
                // Something on screen immediately: an empty frame under a
                // verdict reads as a decoder that failed.
                preview.seek(0.0);
                Some(preview)
            }
            _ => None,
        };

        self.stage = Stage::Loaded(Box::new(Loaded {
            path: path.to_path_buf(),
            source_bytes,
            facts,
            verdict,
            preview,
            at: 0.0,
            trim: Trim::default(),
        }));
    }

    /// Drain the worker and keep the window repainting while anything moves.
    fn pump(&mut self, ctx: &egui::Context) {
        if let Stage::Loaded(loaded) = &self.stage
            && loaded.preview.as_ref().is_some_and(Preview::playing)
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        let Stage::Working(working) = &mut self.stage else {
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(80));

        loop {
            match working.inbox.try_recv() {
                Ok(Note::At(step)) => working.step = step,
                Ok(Note::Done(outcome)) => {
                    self.stage = Stage::Finished(Box::new(Finished {
                        source_bytes: working.source_bytes,
                        outcome: *outcome,
                    }));
                    return;
                }
                // The worker vanished without a verdict. It cannot, but a window
                // that spins forever on a broken invariant is worse than one that
                // says so.
                Err(TryRecvError::Disconnected) => {
                    self.stage = Stage::Finished(Box::new(Finished {
                        source_bytes: working.source_bytes,
                        outcome: Outcome::Failed("the worker stopped unexpectedly".to_owned()),
                    }));
                    return;
                }
                Err(TryRecvError::Empty) => return,
            }
        }
    }

    fn begin(&mut self) {
        let Stage::Loaded(loaded) = &mut self.stage else {
            return;
        };
        let Some(ffmpeg) = self.ffmpeg.clone() else {
            return;
        };
        let Some(facts) = loaded.facts.clone() else {
            return;
        };

        let source = loaded.path.clone();
        let output = source.with_file_name("video.mp4");
        let trim = loaded.trim;
        let forced = trim.moves_the_opening()
            || crate::plan::plan(&facts, trim.allowance(facts.duration)).must_reencode;
        let cancel = Cancel::new();
        let (tx, inbox) = channel();

        std::thread::spawn({
            let cancel = cancel.clone();
            let source = source.clone();
            move || {
                let tx2 = tx.clone();
                let outcome =
                    standardise::standardise(&ffmpeg, &source, &output, trim, &cancel, |step| {
                        let _ = tx2.send(Note::At(step));
                    });
                let _ = tx.send(Note::Done(Box::new(outcome)));
            }
        });

        self.stage = Stage::Working(Box::new(Working {
            path: source,
            source_bytes: loaded.source_bytes,
            cancel,
            inbox,
            step: Step::Reading,
            forced,
        }));
    }
}

// ── the four screens ─────────────────────────────────────────────────────

fn empty(ui: &mut egui::Ui, lang: Lang, ctx: &egui::Context) -> Option<PathBuf> {
    let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
    let available = ui.available_size();

    let (rect, response) = ui.allocate_exact_size(available, egui::Sense::click());
    let stroke = if hovering {
        egui::Stroke::new(2.0, theme::ACCENT)
    } else {
        egui::Stroke::new(1.5, theme::HAIRLINE)
    };
    ui.painter().rect(
        rect.shrink(2.0),
        egui::CornerRadius::same(theme::RADIUS + 6),
        if hovering {
            theme::PANEL_RAISED
        } else {
            theme::PANEL
        },
        stroke,
        egui::StrokeKind::Inside,
    );

    let prompt = if hovering {
        &text::DROP_RELEASE
    } else {
        &text::DROP_PROMPT
    };
    ui.painter().text(
        rect.center() - egui::vec2(0.0, 14.0),
        egui::Align2::CENTER_CENTER,
        prompt.get(lang),
        egui::FontId::proportional(theme::size::HERO),
        if hovering { theme::ACCENT } else { theme::INK },
    );
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 20.0),
        egui::Align2::CENTER_CENTER,
        text::DROP_HINT.get(lang),
        egui::FontId::proportional(theme::size::BODY),
        theme::INK_MUTED,
    );

    // Handed back rather than loaded here, so a picked file and a dropped one
    // take the same path in.
    if response.clicked() {
        return rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "mov", "mkv", "webm", "m4v"])
            .pick_file();
    }
    None
}

impl Press {
    fn loaded(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let lang = self.lang;
        let Stage::Loaded(loaded) = &mut self.stage else {
            return;
        };

        // The picture, if there is one to show.
        if let Some(preview) = &loaded.preview
            && let Some(frame) = preview.frame()
            && (frame.pts != self.shown_pts || self.texture.is_none())
        {
            let image = egui::ColorImage::from_rgba_unmultiplied(
                [frame.width as usize, frame.height as usize],
                &frame.rgba,
            );
            match &mut self.texture {
                Some(handle) => handle.set(image, egui::TextureOptions::LINEAR),
                None => {
                    self.texture =
                        Some(ctx.load_texture("preview", image, egui::TextureOptions::LINEAR));
                }
            }
            self.shown_pts = frame.pts;
        }

        if let Some(texture) = &self.texture {
            let width = ui.available_width().min(560.0);
            let aspect = texture.size()[1] as f32 / texture.size()[0] as f32;
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(texture)
                        .fit_to_exact_size(egui::vec2(width, width * aspect))
                        .corner_radius(egui::CornerRadius::same(theme::RADIUS)),
                );
            });
            ui.add_space(8.0);
            scrubber(ui, lang, loaded);
            ui.add_space(12.0);
        }

        verdict_card(ui, lang, loaded);
        ui.add_space(14.0);

        let can_run = self.ffmpeg.is_some() && loaded.facts.is_some();
        if self.ffmpeg.is_none() {
            ui.label(egui::RichText::new(self.t(&text::NO_ENCODER)).color(theme::ALARM));
        }
        if can_run {
            ui.vertical_centered(|ui| {
                if theme::primary(ui, text::START.get(lang)).clicked() {
                    self.begin();
                }
            });
        }
    }

    fn working(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Stage::Working(working) = &mut self.stage else {
            return;
        };

        theme::card(ui, |ui| {
            ui.label(
                egui::RichText::new(file_name(&working.path))
                    .size(theme::size::SMALL)
                    .color(theme::INK_MUTED),
            );
            ui.add_space(10.0);

            let (label, fraction) = match &working.step {
                Step::Reading => (text::READING.get(lang).to_owned(), None),
                Step::Copying(p) => (
                    text::STEP_COPYING.get(lang).to_owned(),
                    p.and_then(|p| p.fraction),
                ),
                Step::Encoding { progress, .. } => {
                    let fraction = progress.and_then(|p| p.fraction);
                    let template = if working.forced {
                        &text::STEP_ENCODING_FORCED
                    } else {
                        &text::STEP_ENCODING
                    };
                    let percent =
                        fraction.map_or_else(|| "…".to_owned(), |f| format!("{:.0}%", f * 100.0));
                    (template.get(lang).replace("{percent}", &percent), fraction)
                }
                Step::Verifying => (text::STEP_VERIFYING.get(lang).to_owned(), None),
            };

            ui.label(egui::RichText::new(label).size(theme::size::TITLE));
            ui.add_space(8.0);
            let bar = egui::ProgressBar::new(fraction.unwrap_or(0.0) as f32)
                .desired_height(8.0)
                .corner_radius(egui::CornerRadius::same(4))
                .fill(theme::ACCENT);
            ui.add(if fraction.is_some() {
                bar
            } else {
                bar.animate(true)
            });

            ui.add_space(14.0);
            // The same button means two different things, so it says two
            // different things: with a compliant copy already in hand, stopping
            // keeps it; without one, stopping keeps nothing.
            let label = if working.forced {
                &text::CANCEL_ABANDON
            } else {
                &text::CANCEL_KEEP
            };
            if theme::quiet(ui, label.get(lang)).clicked() {
                working.cancel.request();
            }
            if working.forced {
                theme::muted(ui, text::CANCEL_ABANDON_HINT.get(lang));
            }
        });
    }

    fn finished(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Stage::Finished(finished) = &self.stage else {
            return;
        };
        let source_bytes = finished.source_bytes;

        let mut open_folder: Option<PathBuf> = None;
        theme::card(ui, |ui| match &finished.outcome {
            Outcome::Produced(produced) => {
                theme::heading(ui, text::DONE.get(lang), theme::GOOD);
                ui.add_space(6.0);
                ui.label(
                    text::DONE_SIZE
                        .get(lang)
                        .replace("{before}", &megabytes(source_bytes))
                        .replace("{after}", &megabytes(produced.bytes)),
                );
                ui.add_space(4.0);
                for line in outcome_notes(produced, lang) {
                    theme::muted(ui, &line);
                }
                ui.add_space(12.0);
                open_folder = Some(produced.path.clone());
            }
            Outcome::AlreadyStandard => {
                theme::heading(ui, text::ALREADY_STANDARD.get(lang), theme::GOOD);
            }
            Outcome::Cancelled => {
                theme::heading(ui, text::CANCELLED.get(lang), theme::INK_MUTED);
            }
            Outcome::Failed(reason) => {
                theme::heading(ui, text::FAILED.get(lang), theme::ALARM);
                ui.add_space(6.0);
                ui.label(egui::RichText::new(reason).size(theme::size::SMALL));
            }
        });

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            if let Some(path) = open_folder
                && theme::quiet(ui, text::SHOW_IN_FOLDER.get(lang)).clicked()
            {
                reveal(&path);
            }
            if theme::quiet(ui, text::ANOTHER.get(lang)).clicked() {
                self.stage = Stage::Empty;
                self.texture = None;
            }
        });
    }
}

/// The findings, and what is about to be done about each of them.
fn verdict_card(ui: &mut egui::Ui, lang: Lang, loaded: &Loaded) {
    theme::card(ui, |ui| {
        ui.label(
            egui::RichText::new(file_name(&loaded.path))
                .size(theme::size::SMALL)
                .color(theme::INK_MUTED),
        );
        ui.add_space(8.0);

        let Some(facts) = &loaded.facts else {
            theme::heading(ui, text::VERDICT_UNREADABLE.get(lang), theme::ALARM);
            return;
        };

        if loaded.verdict.passes() {
            theme::heading(ui, text::VERDICT_OK.get(lang), theme::GOOD);
        } else {
            theme::heading(ui, text::VERDICT_WILL_FIX.get(lang), theme::NOTICE);
        }
        ui.add_space(4.0);
        theme::muted(ui, &shape(facts, loaded.source_bytes));
        ui.add_space(10.0);

        // Every finding, not only the first. The gate reports one violation
        // because it is refusing; Press is fixing, so it lists all of them --
        // being told about the audio and then, on the next attempt, about the
        // frame rate is the round trip this whole tool exists to remove.
        for (finding, fix) in findings(facts, loaded.source_bytes, lang) {
            ui.horizontal_top(|ui| {
                ui.label(egui::RichText::new("•").color(theme::NOTICE));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(finding).color(theme::INK));
                    theme::muted(ui, &fix);
                });
            });
        }
    });
}

fn findings(facts: &Facts, source_bytes: u64, lang: Lang) -> Vec<(String, String)> {
    let mut out = Vec::new();

    if source_bytes > limits::MAX_BYTES {
        out.push((
            text::FINDING_TOO_LARGE
                .get(lang)
                .replace("{size}", &source_bytes.div_ceil(1024 * 1024).to_string())
                .replace(
                    "{limit}",
                    &limits::MAX_BYTES.div_ceil(1024 * 1024).to_string(),
                ),
            text::FIX_CODEC.get(lang).to_owned(),
        ));
    }
    if !limits::codec_allowed(facts.codec) {
        let named = if facts.raw_fourcc.is_empty() {
            "?"
        } else {
            &facts.raw_fourcc
        };
        out.push((
            text::FINDING_CODEC.get(lang).replace("{codec}", named),
            text::FIX_CODEC.get(lang).to_owned(),
        ));
    }
    if u64::from(facts.width) * u64::from(facts.height) > limits::MAX_PIXELS {
        let target = crate::plan::plan(facts, None)
            .size
            .unwrap_or((facts.width, facts.height));
        out.push((
            text::FINDING_RESOLUTION
                .get(lang)
                .replace("{width}", &facts.width.to_string())
                .replace("{height}", &facts.height.to_string()),
            text::FIX_RESOLUTION
                .get(lang)
                .replace("{width}", &target.0.to_string())
                .replace("{height}", &target.1.to_string()),
        ));
    }
    if crate::gate::probe::over_fps_limit(facts.fps) {
        out.push((
            text::FINDING_FPS
                .get(lang)
                .replace(
                    "{fps}",
                    &crate::gate::probe::reported_fps(facts.fps).to_string(),
                )
                .replace("{limit}", &limits::MAX_FPS.to_string()),
            text::FIX_FPS
                .get(lang)
                .replace("{limit}", &limits::MAX_FPS.to_string()),
        ));
    }
    if facts.has_audio {
        out.push((
            text::FINDING_AUDIO.get(lang).to_owned(),
            text::FIX_AUDIO.get(lang).to_owned(),
        ));
    }
    out
}

/// The timeline, the transport, and what the trim currently means in words.
fn scrubber(ui: &mut egui::Ui, lang: Lang, loaded: &mut Loaded) {
    let Some(facts) = &loaded.facts else { return };
    let duration = facts.duration.max(0.001);

    match crate::timeline::timeline(ui, duration, loaded.at, &mut loaded.trim, true) {
        Change::None => {}
        Change::Seek(at) => {
            loaded.at = at;
            if let Some(preview) = &loaded.preview {
                preview.seek(at);
            }
        }
        // Dragging a handle moves the picture to that handle, because the frame
        // being cut to is the entire reason to look at a preview at all.
        Change::Trim { opening_moved } => {
            let at = if opening_moved {
                loaded.trim.cut_opening
            } else {
                loaded.trim.keep_until.unwrap_or(duration)
            };
            loaded.at = at;
            if let Some(preview) = &loaded.preview {
                preview.seek((at - 0.05).max(0.0));
            }
        }
    }

    ui.horizontal(|ui| {
        if let Some(preview) = &loaded.preview {
            let playing = preview.playing();
            if ui.button(if playing { "⏸" } else { "▶" }).clicked() {
                if playing {
                    preview.pause();
                } else {
                    preview.play(loaded.at);
                }
            }
        }
        theme::muted(ui, &format!("{} / {}", clock(loaded.at), clock(duration)));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if !loaded.trim.is_untouched() && theme::quiet(ui, text::TRIM_RESET.get(lang)).clicked()
            {
                loaded.trim = Trim::default();
            }
        });
    });

    // What the handles currently mean, in words. The picture shows WHICH part
    // survives; only a sentence can say what that costs.
    if loaded.trim.is_untouched() {
        theme::muted(ui, text::TRIM_NONE.get(lang));
        return;
    }

    let kept = loaded.trim.kept_seconds(duration);
    let cut = duration - kept;
    theme::muted(
        ui,
        &text::TRIM_TAIL_ONLY
            .get(lang)
            .replace("{kept}", &clock(kept))
            .replace("{cut}", &clock(cut)),
    );

    // The opening is the half with consequences, so it gets said in full and in
    // the colour that means "read this", not tucked under the same grey line.
    if loaded.trim.moves_the_opening() {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(
                text::TRIM_HEAD_WARNING
                    .get(lang)
                    .replace("{shift}", &clock(loaded.trim.cut_opening)),
            )
            .size(theme::size::SMALL)
            .color(theme::NOTICE),
        );
        theme::muted(ui, text::TRIM_HEAD_COSTS_REENCODE.get(lang));
    }
}

fn outcome_notes(produced: &Produced, lang: Lang) -> Vec<String> {
    let mut notes = Vec::new();
    match &produced.treatment {
        Treatment::StreamCopy => notes.push(text::DONE_UNTOUCHED.get(lang).to_owned()),
        Treatment::KeptOwnCodec { .. } => {
            notes.push(text::DONE_UNTOUCHED.get(lang).to_owned());
            notes.push(text::DONE_AV1_LOST.get(lang).to_owned());
        }
        Treatment::Av1 { .. } => {}
    }
    if produced.av1_cancelled {
        notes.push(text::DONE_AV1_SKIPPED.get(lang).to_owned());
    }
    notes
}

fn shape(facts: &Facts, bytes: u64) -> String {
    format!(
        "{}  ·  {}×{}  ·  {:.0} fps  ·  {}  ·  {}",
        facts.codec.unwrap_or(if facts.raw_fourcc.is_empty() {
            "?"
        } else {
            &facts.raw_fourcc
        }),
        facts.width,
        facts.height,
        facts.fps,
        clock(facts.duration),
        megabytes(bytes),
    )
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    format!("{}:{:04.1}", (seconds / 60.0) as u64, seconds % 60.0)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Opens the folder with the result selected. Best effort: a tool that cannot
/// open a file manager has still done its job.
fn reveal(path: &Path) {
    #[cfg(windows)]
    let _ = std::process::Command::new("explorer")
        .arg(format!("/select,{}", path.display()))
        .spawn();
    #[cfg(not(windows))]
    if let Some(dir) = path.parent() {
        let _ = std::process::Command::new("xdg-open").arg(dir).spawn();
    }
}

/// Not used by the window, but the one thing a verdict must never do is
/// disagree with the gate. Kept honest here so the screen and the file agree.
#[allow(dead_code)]
fn verdict_matches_findings(facts: &Facts, bytes: u64, lang: Lang) -> bool {
    let refused = !gate::check(bytes, Some(facts)).passes();
    refused == !findings(facts, bytes, lang).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The screen lists every finding while the gate reports only the first.
    /// They still have to agree on WHETHER anything is wrong -- a file the gate
    /// refuses with nothing listed, or a file it accepts with a bullet under it,
    /// is a window that contradicts the thing it exists to report.
    #[test]
    fn the_screen_and_the_gate_never_disagree_about_whether_it_passes() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut checked = 0;
        for entry in std::fs::read_dir(dir).expect("fixtures") {
            let path = entry.expect("entry").path();
            if path.extension().is_none_or(|e| e != "mp4") {
                continue;
            }
            let bytes = std::fs::read(&path).expect("fixture");
            let Some(facts) = gate::probe(&bytes) else {
                continue;
            };
            for lang in Lang::ALL {
                assert!(
                    verdict_matches_findings(&facts, bytes.len() as u64, lang),
                    "{} disagrees with the gate in {}",
                    path.display(),
                    lang.endonym()
                );
            }
            checked += 1;
        }
        assert!(checked > 10, "expected the fixture set, found {checked}");
    }

    /// A finding that renders with its braces still in it is a placeholder whose
    /// name was changed on one side only.
    #[test]
    fn no_finding_renders_with_an_unfilled_placeholder() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        for entry in std::fs::read_dir(dir).expect("fixtures") {
            let path = entry.expect("entry").path();
            if path.extension().is_none_or(|e| e != "mp4") {
                continue;
            }
            let bytes = std::fs::read(&path).expect("fixture");
            let Some(facts) = gate::probe(&bytes) else {
                continue;
            };
            for lang in Lang::ALL {
                for (finding, fix) in findings(&facts, bytes.len() as u64, lang) {
                    for rendered in [&finding, &fix] {
                        assert!(
                            !rendered.contains('{') && !rendered.contains('}'),
                            "{}: {rendered}",
                            path.display()
                        );
                    }
                }
            }
        }
    }
}
