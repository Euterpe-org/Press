//! The window.
//!
//! One file at a time, and one thing to press.
//!
//! # The shape, and why it is that shape
//!
//! Three regions, always: a header, a body, and an action bar. **The thing to
//! press is always in the same place** -- "make it uploadable", "stop", "another
//! one" all appear in the bar, so the charter never has to find the button
//! again. The bar is reserved before the body is measured, so nothing can push
//! it off the bottom.
//!
//! The body is two columns: a WORKBENCH on the left -- the picture and the trim
//! -- and a READOUT on the right, which is the verdict. Both fill their column.
//!
//! The window does not resize. Chasing "correct at every size" produced a layout
//! designed for none of them, and a narrow fallback that had to be maintained
//! and was never seen. One canvas means every element is PLACED rather than
//! merely arranged.
//!
//! # Every pixel is accounted for
//!
//! The first version stacked widgets in order and let the leftovers fall where
//! they fell: a preview sized to an arbitrary 560, a verdict card floating in
//! empty space, and content that overflowed with no way to scroll to it. So:
//!
//! * The workbench reserves a FIXED slot for its controls and gives the picture
//!   the rest. Nothing in it changes height for any reason -- the trim warning
//!   used to appear below the timeline and push everything up, which moved the
//!   control being dragged out from under the pointer, mid-drag.
//! * The readout FILLS its column. On a fixed canvas a card that stops halfway
//!   down leaves a rectangle of nothing beside a rectangle of something.
//! * A run in progress, and a finished one, are CENTRED in the body -- there is
//!   nothing else to look at, and pinning a progress bar to the top leaves the
//!   window mostly void.
//! * Four gaps exist (`theme::space`) and each means something.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use eframe::egui;

use crate::ffmpeg::{Cancel, Ffmpeg};
use crate::gate::{self, Facts, limits};
use crate::i18n::{Lang, Text, text};
use crate::phase_a::{self, Applied, Material};
use crate::phase_b::{self, Aim, Encoded, Pending};
use crate::plan::Trim;
use crate::preview::Preview;
use crate::theme::space;
use crate::timeline::Change;
use crate::workspace::Workspace;
use crate::{fonts, theme};

/// Height of the action bar's contents. Reserved before the body is measured.
///
/// `TALL` fits a primary button (44 plus its stroke); `SHORT` fits a quiet one,
/// which is a line of body text inside `button_padding.y` twice. 34 was the
/// guess, and it was two pixels short -- the buttons hung out of the bar and got
/// clipped by the region that contains them.
const ACTION_TALL: f32 = theme::BUTTON_H + 2.0;
const ACTION_SHORT: f32 = ACTION_TALL;

/// The window's own padding, one value so the body's width is derivable.
pub const MARGIN_X: f32 = 24.0;
pub const MARGIN_Y: f32 = 18.0;

/// The workbench and the readout, and the gap between them. They add up to the
/// body's width exactly -- that is the point of a fixed window.
pub const WORKBENCH_W: f32 = 544.0;
pub const READOUT_W: f32 = 340.0;

/// The window's size. **Fixed** -- it does not resize.
///
/// Chasing "correct at every size" produced a layout that was designed for none
/// of them: a preview at an arbitrary width, a card floating in void, a second
/// column that only appeared if somebody dragged the window bigger, and a
/// narrow fallback nobody ever saw but which still had to be maintained. One
/// canvas means every element can be placed rather than merely arranged, and
/// the branch for the size that no longer exists is gone with it.
///
/// 960x640 logical is the largest that still fits a 1366x768 laptop at 100% and
/// a 1920x1080 screen at 150%, which are the two shapes that actually constrain
/// this.
pub const WINDOW: [f32; 2] = [960.0, 640.0];

// Checked by the COMPILER. The two columns and their gap must add up to the
// body EXACTLY -- a fixed canvas whose pieces do not tile it is a fixed canvas
// with a seam or an overlap, and neither is something to discover on screen.
const _: () = assert!(
    WORKBENCH_W + space::SECTION + READOUT_W == WINDOW[0] - 2.0 * MARGIN_X,
    "the columns do not tile the body"
);

pub struct Press {
    lang: Lang,
    ffmpeg: Option<Ffmpeg>,
    stage: Stage,
    texture: Option<egui::TextureHandle>,
    shown_pts: f64,
    /// A file chosen through the picker, waiting to be taken by the same code
    /// path a drop takes. One way in rather than two.
    pending: Option<PathBuf>,
    /// How many times a file has been loaded. Observable because "did that drop
    /// load once or sixty times" is otherwise invisible from outside, and the
    /// answer decides whether a decoder is being started per gesture or per
    /// frame.
    loads: u64,
    /// A failure with no screen to show it on. Displayed over the drop zone
    /// and cleared by the next file.
    trouble: Option<String>,
}

/// The four screens, which are the two phases and the gaps around them.
///
/// `Inspect` and `Bench` are both "here is the situation, what now" screens, and
/// the difference between them is exactly the difference between the phases:
/// Inspect shows what is DECIDED and says nothing about size, because size is
/// not a deterministic question. Bench shows what is left, which is only ever
/// the codec and the weight.
enum Stage {
    Empty,
    Inspect(Box<Inspect>),
    Working(Box<Working>),
    Bench(Box<Bench>),
}

/// A file is loaded and nothing has been done to it.
struct Inspect {
    path: PathBuf,
    bytes: u64,
    facts: Option<Facts>,
    preview: Option<Preview>,
    /// Where the scrubber is, in seconds.
    at: f64,
    /// Which part of the source survives. Untouched by default: the opening is
    /// never moved unless the charter moves it.
    trim: Trim,
}

struct Working {
    /// The charter's own file, for the heading. Never the intermediate: a
    /// progress screen naming a random temp file tells nobody anything.
    source: PathBuf,
    phase: Phase,
    cancel: Cancel,
    inbox: Receiver<Note>,
    /// Held so the temp directory outlives the worker that is writing into it.
    work: Arc<Workspace>,
    /// What the next screen will need, carried by the run itself.
    held: Held,
}

enum Phase {
    A(phase_a::Step),
    B(phase_b::Step),
}

/// Between the phases: what we have, and what is still wrong with it.
///
/// This is the hub. Phase B returns here rather than to a terminal screen,
/// which is what makes "try to make it smaller" an ordinary press of the same
/// button rather than a mode of its own.
struct Bench {
    source: PathBuf,
    source_bytes: u64,
    source_facts: Facts,
    /// Carried from the scrubber. Applied by Phase B, because it needs `-ss`.
    opening_cut: f64,
    /// Phase A's output. **Every** Phase B run encodes from this and never from
    /// a previous Phase B result -- that would be a second generation of loss
    /// for no gain.
    material: Box<Material>,
    /// The best Phase B has produced, if it has produced anything.
    encoded: Option<Box<Encoded>>,
    /// What the last Phase B run concluded, when it did not hand back a file.
    said: Option<Said>,
    /// Where the result was written, once it has been.
    saved: Option<PathBuf>,
    work: Arc<Workspace>,
}

/// A Phase B run that ended without a better file, and why.
enum Said {
    /// AV1 was tried and came back bigger. The lossless material ships.
    MaterialWins {
        av1_was: Option<u64>,
    },
    /// Not even the coarsest quality fits. The remaining lever is length.
    StillTooLarge {
        smallest: u64,
    },
    Cancelled,
    Failed(String),
}

impl Bench {
    /// The file that would be written if the charter saved right now.
    fn have(&self) -> (&Path, u64, &Facts) {
        match &self.encoded {
            Some(done) => (&done.path, done.bytes, &done.facts),
            None => (
                self.material.path(),
                self.material.bytes,
                &self.material.facts,
            ),
        }
    }

    /// What is still wrong. Empty means the file is uploadable as it stands.
    fn pending(&self) -> Vec<Pending> {
        let (_, bytes, facts) = self.have();
        // The opening cut counts as outstanding until an encode has applied it.
        let cut = if self.encoded.is_some() {
            0.0
        } else {
            self.opening_cut
        };
        phase_b::pending(facts, bytes, cut)
    }

    /// Whether pressing the button again could change anything.
    ///
    /// Once Phase B has handed back a file, Press has given its answer -- there
    /// is no second opinion to ask for, because the quality was not a guess.
    fn worth_running(&self) -> Option<Aim> {
        if self.encoded.is_some() {
            return None;
        }
        phase_b::aim(&self.material.facts, self.material.bytes, self.opening_cut)
    }
}

enum Note {
    Step(Phase),
    SettledA(Box<phase_a::Outcome>),
    FittedB(Box<phase_b::Outcome>),
}

/// What the charter pressed on the bench.
enum Act {
    Abandon,
    BackToTrim,
    Run,
    Save,
    Reveal,
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
            loads: 0,
            trouble: None,
        }
    }

    /// Load a file the window was started with -- dropped on the exe, or opened
    /// "with" Press. Queued rather than read here so it takes the same path in
    /// as a drop, on the first frame, once there is a context to draw into.
    pub fn open(&mut self, path: &Path) {
        self.pending = Some(path.to_path_buf());
    }

    /// How many files this window has loaded since it opened.
    pub fn loads_so_far(&self) -> u64 {
        self.loads
    }

    fn t(&self, text: &'static Text) -> &'static str {
        text.get(self.lang)
    }
}

/// What this build calls itself. Taken from the manifest so there is exactly one
/// place a version number exists.
pub const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// The header's height: the tallest thing in it, which is the language button.
/// `button_padding.y` twice plus a line of body text, plus its stroke. Measured
/// rather than guessed -- 34 was the guess, and the debug assertion in `region`
/// caught it laying out 36 on the first run.
const HEADER_H: f32 = 36.0;

impl eframe::App for Press {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.absorb_drops(&ctx);
        self.pump(&ctx);

        // The regions are CUT OUT of the window rather than stacked into it.
        //
        // Stacking, with `set_min_size` at each level to make things fill, was
        // the previous approach and it compounded: measured on a 960x640 window
        // it laid out 1715x1084 of content, every nested minimum pushing its
        // parent wider. On a fixed canvas the rectangles are known in advance,
        // so they are computed once and each region is drawn into exactly its
        // own -- nothing can grow, because nothing is asked how big it wants to
        // be.
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0, theme::BACKDROP);
        let inner = full.shrink2(egui::vec2(MARGIN_X, MARGIN_Y));

        let header = slice_top(inner, HEADER_H);
        let rule_y = header.bottom() + space::ROW;

        let action_h = self.action_height();
        let body_bottom = if action_h > 0.0 {
            inner.bottom() - action_h - space::ROW - 1.0 - space::BLOCK
        } else {
            inner.bottom()
        };
        let body = egui::Rect::from_min_max(
            egui::pos2(inner.left(), rule_y + 1.0 + space::BLOCK),
            egui::pos2(inner.right(), body_bottom),
        );

        hairline(ui, inner.x_range(), rule_y);
        region(ui, "the header", header, |ui| self.header(ui));
        region(ui, "the body", body, |ui| self.body(ui, &ctx));

        if action_h > 0.0 {
            let bar = slice_bottom(inner, action_h);
            hairline(ui, inner.x_range(), bar.top() - space::ROW);
            region(ui, "the action bar", bar, |ui| self.action(ui));
        }
    }
}

fn slice_top(rect: egui::Rect, height: f32) -> egui::Rect {
    egui::Rect::from_min_max(rect.min, egui::pos2(rect.right(), rect.top() + height))
}

fn slice_bottom(rect: egui::Rect, height: f32) -> egui::Rect {
    egui::Rect::from_min_max(egui::pos2(rect.left(), rect.bottom() - height), rect.max)
}

fn hairline(ui: &egui::Ui, x: egui::Rangef, y: f32) {
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
fn region(ui: &mut egui::Ui, name: &str, rect: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
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

impl Press {
    fn action_height(&self) -> f32 {
        match &self.stage {
            Stage::Empty => 0.0,
            Stage::Inspect(inspect) => {
                if self.ffmpeg.is_some() && inspect.facts.is_some() {
                    ACTION_TALL
                } else {
                    0.0
                }
            }
            // Stopping is always safe now: Phase A's material exists before
            // Phase B ever starts, so there is no "you will lose everything"
            // sentence to make room for.
            Stage::Working(_) => ACTION_SHORT,
            Stage::Bench(_) => ACTION_TALL,
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        match &mut self.stage {
            Stage::Empty => {
                let trouble = self.trouble.clone();
                if let Some(picked) = drop_zone(ui, self.lang, ctx, trouble.as_deref()) {
                    self.pending = Some(picked);
                }
            }
            Stage::Inspect(_) => self.inspect(ui, ctx),
            Stage::Working(_) => self.working(ui),
            Stage::Bench(_) => self.bench(ui),
        }
    }

    /// The one thing to press, wherever the window happens to be.
    fn action(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        match &self.stage {
            Stage::Empty => {}
            Stage::Inspect(inspect) => {
                if self.ffmpeg.is_some() && inspect.facts.is_some() {
                    ui.vertical_centered(|ui| {
                        if theme::primary(ui, text::START.get(lang)).clicked() {
                            self.start_phase_a();
                        }
                    });
                }
            }
            Stage::Working(working) => {
                let cancel = working.cancel.clone();
                ui.vertical_centered(|ui| {
                    if theme::quiet(ui, text::STOP.get(lang)).clicked() {
                        cancel.request();
                    }
                });
            }
            Stage::Bench(bench) => {
                let ready = bench.pending().is_empty();
                let runnable = bench.worth_running();
                let stuck = matches!(bench.said, Some(Said::StillTooLarge { .. }));
                let saved = bench.saved.clone();
                let mut pressed = None;

                // A row, centred properly. `vertical_centered` centres a single
                // widget but not a `horizontal` inside it -- the row claims the
                // full width and its contents start hard against the left edge.
                centred_row(ui, "bench-actions", |ui| {
                    if theme::quiet(ui, text::ABANDON.get(lang)).clicked() {
                        pressed = Some(Act::Abandon);
                    }
                    // Only where it is actionable: the message above says to
                    // trim it shorter, and without this there is nowhere to do
                    // that but drag the file in again.
                    if stuck && theme::quiet(ui, text::BACK_TO_TRIM.get(lang)).clicked() {
                        pressed = Some(Act::BackToTrim);
                    }
                    // THE LOUD BUTTON IS ALWAYS THE NEXT USEFUL THING, and it
                    // MOVES as the run progresses -- which is itself a signal
                    // that something happened.
                    //
                    // It used to be pinned to Save, so on a file that was
                    // already fine the recommended action -- make it smaller,
                    // which is smaller for every player who downloads it -- was
                    // the grey pill beside a big blue button. "我找试试压得更小
                    // 这个按钮找了半天."
                    //
                    // Hidden when stuck: the search already reached the coarsest
                    // quality there is, so pressing again spends minutes on the
                    // same answer. The way out is a shorter video.
                    let offer_run = runnable.is_some() && !stuck;
                    if offer_run {
                        let label = if ready {
                            text::SHRINK_FURTHER.get(lang)
                        } else {
                            text::START.get(lang)
                        };
                        if theme::primary(ui, label).clicked() {
                            pressed = Some(Act::Run);
                        }
                    }
                    if ready {
                        // Quiet only while there is something better to press.
                        let hit = if offer_run {
                            theme::quiet(ui, text::SAVE.get(lang)).clicked()
                        } else {
                            theme::primary(ui, text::SAVE.get(lang)).clicked()
                        };
                        if hit {
                            pressed = Some(Act::Save);
                        }
                    }
                    // Only once there is somewhere to go. Uploading is the next
                    // thing that happens, and it starts in a file manager.
                    if ready
                        && saved.is_some()
                        && theme::quiet(ui, text::SHOW_IN_FOLDER.get(lang)).clicked()
                    {
                        pressed = Some(Act::Reveal);
                    }
                });

                match pressed {
                    Some(Act::Abandon) => {
                        // Dropping the stage drops the Arc, and the last Arc
                        // takes the temp directory with it.
                        self.stage = Stage::Empty;
                        self.texture = None;
                    }
                    Some(Act::BackToTrim) => self.back_to_inspect(),
                    Some(Act::Run) => {
                        if let Some(aim) = runnable {
                            self.start_phase_b(aim);
                        }
                    }
                    Some(Act::Save) => self.save(),
                    Some(Act::Reveal) => {
                        if let Some(to) = saved {
                            reveal(&to);
                        }
                    }
                    None => {}
                }
            }
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(self.t(&text::WINDOW_TITLE))
                    .size(theme::size::TITLE)
                    .strong()
                    .color(theme::INK),
            );
            ui.add_space(space::TIGHT);
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

                // The version, beside the only other thing in the header that is
                // about Press rather than about the file. Small and muted: it is
                // never what anyone came here for, but it is the first thing
                // asked for when a charter reports that something misbehaved.
                ui.add_space(space::ROW);
                theme::muted(ui, VERSION);
            });
        });
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
        self.loads += 1;
        // A new file is a new situation; whatever went wrong with the last one
        // is no longer news.
        self.trouble = None;
        self.texture = None;
        self.shown_pts = f64::NAN;

        let Ok(bytes) = std::fs::read(path) else {
            self.stage = Stage::Empty;
            return;
        };
        let source_bytes = bytes.len() as u64;
        let (facts, _) = gate::judge(&bytes);
        drop(bytes);

        let preview = match (&facts, &self.ffmpeg) {
            (Some(f), Some(ffmpeg)) => {
                let preview = Preview::open(ffmpeg.binary(), path, f.width, f.height, f.fps);
                // Something on screen immediately: an empty frame under a
                // verdict reads as a decoder that failed.
                preview.seek(0.0);
                Some(preview)
            }
            _ => None,
        };

        self.stage = Stage::Inspect(Box::new(Inspect {
            path: path.to_path_buf(),
            bytes: source_bytes,
            facts,
            preview,
            at: 0.0,
            trim: Trim::default(),
        }));
    }

    /// Drain the worker and keep the window repainting while anything moves.
    fn pump(&mut self, ctx: &egui::Context) {
        if let Stage::Inspect(loaded) = &mut self.stage
            && let Some(preview) = &loaded.preview
        {
            // Decoded frames arrive on a worker thread, and egui only redraws
            // when something asks it to. Without this the first frame appears
            // when the charter happens to move the mouse -- which looked like it
            // worked, right up until a screenshot caught a window with no
            // picture in it. A viewfinder that needs to be nudged is broken.
            let interval = if preview.playing() { 16 } else { 90 };
            ctx.request_repaint_after(std::time::Duration::from_millis(interval));

            // THE PLAYHEAD FOLLOWS THE PICTURE. It did not: `at` only ever moved
            // when the charter moved it, so pressing play started the video and
            // left the marker and the clock sitting exactly where they were --
            // a transport that transports nothing. The decoded frame knows where
            // it is, so that is what the position now is.
            if preview.playing()
                && let Some(frame) = preview.frame()
            {
                loaded.at = frame.pts;

                // Playback stops at the trim, because past the trim is material
                // the charter has already decided nobody will see. A preview
                // that runs on into it is previewing a different video.
                if let Some(end) = loaded.trim.keep_until
                    && frame.pts >= end
                {
                    preview.pause();
                    loaded.at = end;
                }
            }
        }

        let Stage::Working(working) = &mut self.stage else {
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(80));

        loop {
            match working.inbox.try_recv() {
                Ok(Note::Step(phase)) => working.phase = phase,
                Ok(Note::SettledA(outcome)) => {
                    self.finish_phase_a(*outcome);
                    return;
                }
                Ok(Note::FittedB(outcome)) => {
                    self.finish_phase_b(*outcome);
                    return;
                }
                // The worker vanished without a verdict. It cannot, but a window
                // that spins forever on a broken invariant is worse than one that
                // says so.
                Err(TryRecvError::Disconnected) => {
                    self.give_up("the worker stopped unexpectedly");
                    return;
                }
                Err(TryRecvError::Empty) => return,
            }
        }
    }

    /// Phase A: the deterministic pass. Nothing is asked of the charter here --
    /// every answer follows from the file.
    fn start_phase_a(&mut self) {
        let Stage::Inspect(inspect) = &self.stage else {
            return;
        };
        let (Some(ffmpeg), Some(facts)) = (self.ffmpeg.clone(), inspect.facts.clone()) else {
            return;
        };
        let source = inspect.path.clone();
        let held = Held {
            source_bytes: inspect.bytes,
            source_facts: facts.clone(),
            trim: inspect.trim,
            material: None,
            encoded: None,
        };

        let work = match Workspace::open() {
            Ok(work) => Arc::new(work),
            Err(error) => {
                self.give_up(&format!("no temporary directory to work in: {error}"));
                return;
            }
        };

        let cancel = Cancel::new();
        let (tx, inbox) = channel();
        std::thread::spawn({
            let cancel = cancel.clone();
            let source = source.clone();
            let work = Arc::clone(&work);
            let bytes = held.source_bytes;
            let trim = held.trim;
            move || {
                let tx2 = tx.clone();
                let outcome = phase_a::run(
                    &ffmpeg,
                    &source,
                    &facts,
                    bytes,
                    trim,
                    &work,
                    &cancel,
                    |step| {
                        let _ = tx2.send(Note::Step(Phase::A(step)));
                    },
                );
                let _ = tx.send(Note::SettledA(Box::new(outcome)));
            }
        });

        self.stage = Stage::Working(Box::new(Working {
            source,
            phase: Phase::A(phase_a::Step::Reading),
            cancel,
            inbox,
            work,
            held,
        }));
    }

    fn finish_phase_a(&mut self, outcome: phase_a::Outcome) {
        let Stage::Working(working) = &mut self.stage else {
            return;
        };
        match outcome {
            phase_a::Outcome::Settled(material) => {
                working.held.material = Some(material);
                self.show_bench(None);
            }
            // Nothing was produced and nothing was lost, so the charter goes
            // back to exactly where they pressed the button -- trim included.
            phase_a::Outcome::Cancelled => self.back_to_inspect_from_work(),
            phase_a::Outcome::Failed(reason) => self.give_up(&reason),
        }
    }

    /// Phase B: the fight with 80 MB. `aim` is what the bench decided it is for.
    fn start_phase_b(&mut self, aim: Aim) {
        if !matches!(self.stage, Stage::Bench(_)) {
            return;
        }
        let Some(ffmpeg) = self.ffmpeg.clone() else {
            return;
        };
        let Stage::Bench(bench) = std::mem::replace(&mut self.stage, Stage::Empty) else {
            unreachable!("checked above");
        };
        let bench = *bench;
        let opening_cut = bench.opening_cut;
        let work = Arc::clone(&bench.work);

        // The worker gets its own copy. `Material` is a path and some numbers,
        // so this costs nothing and it means no lifetime has to cross a thread
        // boundary for the sake of a struct that could have been cloned.
        let material = (*bench.material).clone();

        let cancel = Cancel::new();
        let (tx, inbox) = channel();
        std::thread::spawn({
            let cancel = cancel.clone();
            let work = Arc::clone(&work);
            move || {
                let tx2 = tx.clone();
                let outcome = phase_b::run(
                    &ffmpeg,
                    &material,
                    aim,
                    opening_cut,
                    &work,
                    &cancel,
                    |step| {
                        let _ = tx2.send(Note::Step(Phase::B(step)));
                    },
                );
                let _ = tx.send(Note::FittedB(Box::new(outcome)));
            }
        });

        self.stage = Stage::Working(Box::new(Working {
            source: bench.source,
            phase: Phase::B(phase_b::Step::Encoding {
                crf: crate::plan::CRF,
                attempt: 1,
                progress: None,
            }),
            cancel,
            inbox,
            work,
            held: Held {
                source_bytes: bench.source_bytes,
                source_facts: bench.source_facts,
                trim: Trim {
                    cut_opening: opening_cut,
                    keep_until: None,
                },
                material: Some(bench.material),
                encoded: bench.encoded,
            },
        }));
    }

    fn finish_phase_b(&mut self, outcome: phase_b::Outcome) {
        let Stage::Working(working) = &mut self.stage else {
            return;
        };
        let said = match outcome {
            // A better file. Anything Phase B produced before it is superseded,
            // and the workspace will take the loser with it.
            phase_b::Outcome::Encoded(done) => {
                working.held.encoded = Some(done);
                None
            }
            phase_b::Outcome::MaterialWins { av1_was } => Some(Said::MaterialWins { av1_was }),
            phase_b::Outcome::StillTooLarge { smallest } => Some(Said::StillTooLarge { smallest }),
            phase_b::Outcome::Cancelled => Some(Said::Cancelled),
            phase_b::Outcome::Failed(reason) => Some(Said::Failed(reason)),
        };
        self.show_bench(said);
    }

    /// Rebuild the bench from the run that just finished.
    fn show_bench(&mut self, said: Option<Said>) {
        let Stage::Working(working) = std::mem::replace(&mut self.stage, Stage::Empty) else {
            return;
        };
        let working = *working;
        let Some(material) = working.held.material else {
            // Only reachable if Phase A reported success without a material,
            // which it cannot.
            self.give_up("the run finished without producing anything");
            return;
        };
        self.stage = Stage::Bench(Box::new(Bench {
            source: working.source,
            source_bytes: working.held.source_bytes,
            source_facts: working.held.source_facts,
            opening_cut: working.held.trim.cut_opening,
            material,
            encoded: working.held.encoded,
            said,
            saved: None,
            work: working.work,
        }));
    }

    /// Write the current best somewhere the charter chooses.
    ///
    /// This is the ONLY moment a destination is asked for. Everything before it
    /// went to a temp directory under a name nobody had to approve -- asking
    /// where to put a file that might never exist is the wrong question at the
    /// wrong time, and answering it early is how Press used to overwrite
    /// whatever was already called `video.mp4`.
    fn save(&mut self) {
        let Stage::Bench(bench) = &self.stage else {
            return;
        };
        let (from, _, _) = bench.have();
        let from = from.to_path_buf();
        let Some(to) = rfd::FileDialog::new()
            .set_title(text::SAVE_TITLE.get(self.lang))
            .set_directory(bench.source.parent().unwrap_or(Path::new(".")))
            .set_file_name("video.mp4")
            .add_filter("MP4", &["mp4"])
            .save_file()
        else {
            return;
        };
        if same_file(&from, &to) {
            return;
        }
        // Copied rather than renamed: the workspace lives on the temp volume,
        // which is usually not the volume the charter picked. The system dialog
        // has already asked about overwriting, in their own Windows language.
        match std::fs::copy(&from, &to) {
            Ok(_) => {
                if let Stage::Bench(bench) = &mut self.stage {
                    bench.saved = Some(to);
                }
            }
            Err(error) => self.give_up(&format!("could not write the result: {error}")),
        }
    }

    /// Something went wrong that no screen can offer a next step for.
    fn give_up(&mut self, reason: &str) {
        self.trouble = Some(reason.to_owned());
        self.stage = Stage::Empty;
        self.texture = None;
    }

    /// Back to the scrubber from a cancelled run, with the trim intact.
    fn back_to_inspect_from_work(&mut self) {
        let Stage::Working(working) = std::mem::replace(&mut self.stage, Stage::Empty) else {
            return;
        };
        let working = *working;
        self.reopen(
            working.source,
            working.held.source_bytes,
            working.held.source_facts,
            working.held.trim,
        );
    }

    /// Back to the scrubber from the bench, keeping the opening cut so the
    /// advice to "trim it shorter" has somewhere to be taken.
    fn back_to_inspect(&mut self) {
        let Stage::Bench(bench) = std::mem::replace(&mut self.stage, Stage::Empty) else {
            return;
        };
        let bench = *bench;
        self.reopen(
            bench.source,
            bench.source_bytes,
            bench.source_facts,
            Trim {
                cut_opening: bench.opening_cut,
                keep_until: None,
            },
        );
    }

    fn reopen(&mut self, source: PathBuf, bytes: u64, facts: Facts, trim: Trim) {
        let preview = self.ffmpeg.as_ref().map(|ffmpeg| {
            let preview = Preview::open(
                ffmpeg.binary(),
                &source,
                facts.width,
                facts.height,
                facts.fps,
            );
            preview.seek(0.0);
            preview
        });
        self.texture = None;
        self.shown_pts = f64::NAN;
        self.stage = Stage::Inspect(Box::new(Inspect {
            path: source,
            bytes,
            facts: Some(facts),
            preview,
            at: 0.0,
            trim,
        }));
    }
}

/// What a run carries on behalf of the screen that comes after it.
///
/// The worker knows nothing about screens, and the screen it came from is gone
/// while it runs -- so everything the next one needs travels with the run.
struct Held {
    source_bytes: u64,
    source_facts: Facts,
    trim: Trim,
    /// Phase A's output. `None` only while Phase A is still producing it.
    material: Option<Box<Material>>,
    /// The best Phase B result so far, kept across a run that does worse.
    encoded: Option<Box<Encoded>>,
}

/// Whether two paths name the same file on disk.
///
/// Compared after canonicalising, because `.\video.mp4` and an absolute path to
/// it are the same file and a string comparison says otherwise. A path that
/// cannot be canonicalised does not exist yet, which answers the question.
fn same_file(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

// ── the four bodies ──────────────────────────────────────────────────────

/// Fills the body, because there is nothing else in it and a drop target that
/// does not look like the whole window is a smaller target than it needs to be.
fn drop_zone(
    ui: &mut egui::Ui,
    lang: Lang,
    ctx: &egui::Context,
    trouble: Option<&str>,
) -> Option<PathBuf> {
    let hovering = ctx.input(|i| !i.raw.hovered_files.is_empty());
    let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
    // Windows refuses to let a medium-integrity Explorer drop onto a high-
    // integrity window, and the entire visible result is a forbidden cursor --
    // no message, no log, nothing but a tool that appears broken. Clicking still
    // works, so the zone says which half of itself is dead and why. See
    // `crate::elevation`.
    let blocked = crate::elevation::is_elevated();

    ui.painter().rect(
        rect.shrink(1.0),
        egui::CornerRadius::same(theme::RADIUS + 6),
        if hovering {
            theme::PANEL_RAISED
        } else {
            theme::PANEL
        },
        if hovering {
            egui::Stroke::new(2.0, theme::ACCENT)
        } else {
            egui::Stroke::new(1.5, theme::HAIRLINE)
        },
        egui::StrokeKind::Inside,
    );

    // When dragging is blocked the headline stops promising it: "click to choose
    // a video" is the only instruction that is actually true, and the sentence
    // under it explains why the other half is missing.
    let prompt = match (blocked, hovering) {
        (true, _) => &text::DROP_PICK_INSTEAD,
        (false, true) => &text::DROP_RELEASE,
        (false, false) => &text::DROP_PROMPT,
    };
    // The pair sits on the optical centre rather than the geometric one: two
    // lines centred as a block read as one thing, which is what they are.
    ui.painter().text(
        rect.center() - egui::vec2(0.0, space::ROW),
        egui::Align2::CENTER_CENTER,
        prompt.get(lang),
        egui::FontId::proportional(theme::size::HERO),
        if hovering && !blocked {
            theme::ACCENT
        } else {
            theme::INK
        },
    );
    if blocked {
        wrapped_centre(
            ui,
            rect.center() + egui::vec2(0.0, space::BLOCK),
            text::DROP_BLOCKED_BY_ADMIN.get(lang),
            theme::NOTICE,
            rect.width().min(460.0),
        );
    } else {
        ui.painter().text(
            rect.center() + egui::vec2(0.0, space::BLOCK),
            egui::Align2::CENTER_CENTER,
            text::DROP_HINT.get(lang),
            egui::FontId::proportional(theme::size::BODY),
            theme::INK_MUTED,
        );
    }

    // A run that ended somewhere no screen can offer a next step from. It says
    // so HERE rather than nowhere -- dropping someone back to an empty window
    // with no explanation is the failure mode that sends them hunting.
    if let Some(reason) = trouble {
        wrapped_centre(
            ui,
            rect.center() + egui::vec2(0.0, space::SECTION * 2.0),
            reason,
            theme::ALARM,
            rect.width().min(520.0),
        );
    }

    // Handed back rather than loaded here, so a picked file and a dropped one
    // take the same path in. The click works whatever the integrity level is,
    // which is the whole reason the elevated case has something to say.
    if response.clicked() {
        return rfd::FileDialog::new()
            .add_filter("Video", &["mp4", "mov", "mkv", "webm", "m4v"])
            .pick_file();
    }
    None
}

/// A sentence painted centred and wrapped, for the one place with no widget to
/// hang it off -- the drop zone is a single painted rectangle by design.
fn wrapped_centre(
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

impl Press {
    fn inspect(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.refresh_texture(ctx);
        let lang = self.lang;
        let texture = self.texture.clone();
        let no_encoder = self.ffmpeg.is_none();
        let Stage::Inspect(loaded) = &mut self.stage else {
            return;
        };

        let body = ui.max_rect();
        let workbench = egui::Rect::from_min_max(
            body.min,
            egui::pos2(body.left() + WORKBENCH_W, body.bottom()),
        );
        let readout =
            egui::Rect::from_min_max(egui::pos2(body.right() - READOUT_W, body.top()), body.max);

        region(ui, "the workbench", workbench, |ui| {
            workbench_column(ui, lang, loaded, texture.as_ref());
        });
        region(ui, "the readout", readout, |ui| {
            verdict_card(ui, lang, loaded, no_encoder);
        });
    }

    fn refresh_texture(&mut self, ctx: &egui::Context) {
        let Stage::Inspect(loaded) = &self.stage else {
            return;
        };
        let Some(preview) = &loaded.preview else {
            return;
        };
        let Some(frame) = preview.frame() else {
            return;
        };
        if frame.pts == self.shown_pts && self.texture.is_some() {
            return;
        }

        let image = egui::ColorImage::from_rgba_unmultiplied(
            [frame.width as usize, frame.height as usize],
            &frame.rgba,
        );
        match &mut self.texture {
            Some(handle) => handle.set(image, egui::TextureOptions::LINEAR),
            None => {
                self.texture =
                    Some(ctx.load_texture("preview", image, egui::TextureOptions::LINEAR))
            }
        }
        self.shown_pts = frame.pts;
    }

    /// Centred in the body: there is nothing else to look at, and a progress bar
    /// pinned to the top leaves the rest of the window empty for no reason.
    fn working(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Stage::Working(working) = &self.stage else {
            return;
        };

        let (label, fraction, written) = match &working.phase {
            Phase::A(phase_a::Step::Reading) => (text::READING.get(lang).to_owned(), None, None),
            Phase::A(phase_a::Step::Copying(p)) => (
                text::STEP_COPYING.get(lang).to_owned(),
                p.and_then(|p| p.fraction),
                p.map(|p| p.bytes),
            ),
            Phase::A(phase_a::Step::Verifying) | Phase::B(phase_b::Step::Verifying) => {
                (text::STEP_VERIFYING.get(lang).to_owned(), None, None)
            }
            Phase::B(phase_b::Step::Encoding { crf, progress, .. }) => {
                let fraction = progress.and_then(|p| p.fraction);
                let percent =
                    fraction.map_or_else(|| "…".to_owned(), |f| format!("{:.0}%", f * 100.0));
                (
                    text::STEP_ENCODING
                        .get(lang)
                        .replace("{percent}", &percent)
                        .replace("{crf}", &crf.to_string()),
                    fraction,
                    progress.map(|p| p.bytes),
                )
            }
            Phase::B(phase_b::Step::Recalibrating { from, to }) => (
                text::STEP_RECALIBRATING
                    .get(lang)
                    .replace("{from}", &from.to_string())
                    .replace("{to}", &to.to_string()),
                None,
                None,
            ),
        };
        let name = working.source.to_string_lossy().into_owned();

        centred(ui, "working", |ui| {
            theme::card(ui, |ui| {
                ui.set_width(ui.available_width());
                theme::path(ui, &name);
                ui.add_space(space::ROW);
                ui.label(egui::RichText::new(label).size(theme::size::TITLE));
                ui.add_space(space::ROW);
                let bar = egui::ProgressBar::new(fraction.unwrap_or(0.0) as f32)
                    .desired_height(8.0)
                    .corner_radius(egui::CornerRadius::same(4))
                    .fill(theme::ACCENT);
                ui.add(if fraction.is_some() {
                    bar
                } else {
                    bar.animate(true)
                });

                // THE SIZE, as it is written. The percentage says how far the
                // encoder has got; the size says whether it is going to be any
                // use, which is the question the charter actually has -- and it
                // is what the ceilings below are watching too.
                if let Some(bytes) = written {
                    ui.add_space(space::ROW);
                    ui.label(
                        egui::RichText::new(
                            text::STEP_WRITTEN
                                .get(lang)
                                .replace("{size}", &megabytes(bytes))
                                .replace("{limit}", &megabytes(limits::MAX_BYTES)),
                        )
                        .monospace()
                        .size(theme::size::SMALL)
                        .color(if bytes > limits::MAX_BYTES {
                            theme::NOTICE
                        } else {
                            theme::INK_MUTED
                        }),
                    );
                }
            });
        });
    }

    /// The hub between the phases: what we have, and what is still wrong.
    ///
    /// ONE card, centred, sized to its contents. The first version was two
    /// full-height columns and it was a mess: both cards opened with a green
    /// heading saying the same thing in different words, one of them showed the
    /// path of a temp file nobody should ever see, and each was a border drawn
    /// around four hundred pixels of nothing.
    ///
    /// So the screen has one focal point and a strict order under it -- the
    /// verdict, the number, then everything else in one muted line. Whatever is
    /// left over is left empty, because a card stretched around emptiness is
    /// worse than emptiness.
    fn bench(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Stage::Bench(bench) = &self.stage else {
            return;
        };
        let pending = bench.pending();
        let (_, bytes, facts) = bench.have();

        centred(ui, "bench", |ui| {
            theme::card(ui, |ui| {
                ui.set_width(ui.available_width());

                // Which file this is about -- the charter's own, never the
                // intermediate. Elided rather than wrapped: three lines of
                // monospace above the headline buries the headline.
                theme::muted(ui, &elide_middle(&bench.source.to_string_lossy(), 64));
                ui.add_space(space::ROW);

                // THE ONE THING TO READ FIRST, and it reports the EVENT where
                // there has just been one.
                //
                // It used to report the state, and the state after a successful
                // encode reads the same as the state before it -- "ready to
                // upload" was true both times. Coming back to a screen whose
                // headline has not changed is indistinguishable from nothing
                // having happened, and that is exactly how it was read: "我看了
                // 半天才意识到哦原来是压完了".
                let (hero, colour) =
                    match (&bench.said, bench.encoded.is_some(), pending.is_empty()) {
                        (Some(Said::Failed(_)), ..) => (text::FAILED.get(lang), theme::ALARM),
                        (Some(Said::StillTooLarge { .. }), ..) => {
                            (text::STILL_TOO_LARGE.get(lang), theme::NOTICE)
                        }
                        (_, true, true) => (text::BENCH_COMPRESSED.get(lang), theme::GOOD),
                        (_, _, true) => (text::BENCH_READY.get(lang), theme::GOOD),
                        (_, _, false) => (text::BENCH_REMAINING.get(lang), theme::NOTICE),
                    };
                theme::heading(ui, hero, colour);

                // THE NUMBER, at the size a number deserves -- but only once
                // there is a before AND an after. Until something has been
                // produced the two are the same file, and "28.4 MB → 28.4 MB"
                // in eighteen point is a headline that says nothing while
                // taking the space of the list that says everything.
                ui.add_space(space::ROW);
                if bytes == bench.source_bytes {
                    theme::muted(
                        ui,
                        &format!("{}  ·  {}", megabytes(bytes), shape_only(facts)),
                    );
                } else {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} → {}",
                            megabytes(bench.source_bytes),
                            megabytes(bytes)
                        ))
                        .size(theme::size::TITLE)
                        .color(theme::INK),
                    );
                    ui.add_space(2.0);

                    // "87% smaller" is a number that was not on the previous
                    // screen, which is half of why this one now reads as a
                    // different screen rather than the same one again.
                    let mut under = Vec::new();
                    if bytes < bench.source_bytes && bench.source_bytes > 0 {
                        let saved = 100.0 - bytes as f64 / bench.source_bytes as f64 * 100.0;
                        under.push(
                            text::BENCH_SMALLER
                                .get(lang)
                                .replace("{percent}", &format!("{saved:.0}")),
                        );
                    }
                    under.push(
                        text::BENCH_ALLOWANCE
                            .get(lang)
                            .replace(
                                "{percent}",
                                &format!("{:.0}", bytes as f64 / limits::MAX_BYTES as f64 * 100.0),
                            )
                            .replace("{limit}", &megabytes(limits::MAX_BYTES)),
                    );
                    theme::muted(ui, &under.join("  ·  "));
                }

                // The list and the verdict are ALTERNATIVES, not a stack.
                //
                // "Still to fix: 379 MB, over the 80 MB limit -- will be
                // compressed to under 80 MB" under a headline that says it
                // cannot be is both a repetition and a lie. Where a run has
                // already answered, its answer is the message.
                match &bench.said {
                    Some(said @ (Said::StillTooLarge { .. } | Said::Failed(_))) => {
                        ui.add_space(space::ROW);
                        said_line(ui, lang, said);
                    }
                    other => {
                        for (what, fix) in pending_lines(&pending, lang) {
                            // A LIST, spaced as one. `ROW` between items that
                            // are each two lines apart makes five findings read
                            // as five separate announcements, and costs sixty
                            // pixels that the fifth finding then does not have.
                            ui.add_space(space::TIGHT);
                            // The bullet is part of the sentence rather than a
                            // column beside it. A `horizontal` row holding a
                            // marker and a block of text puts the text in a
                            // layout that resolves its wrap mode to EXTEND, and
                            // the long ones then ran off the card.
                            theme::line(ui, &format!("•  {what}"), theme::INK);
                            ui.add_space(2.0);
                            theme::muted(ui, &format!("    {fix}"));
                        }
                        if let Some(said) = other {
                            ui.add_space(space::ROW);
                            said_line(ui, lang, said);
                        }
                    }
                }

                // One muted line for the whole of what was done and how. It is
                // worth stating -- Press chose these and nobody else can -- but
                // it is never what anyone opened the window to find out.
                ui.add_space(space::BLOCK);
                theme::rule(ui);
                ui.add_space(space::TIGHT);
                theme::muted(ui, &done_line(bench, facts, lang));

                if let Some(to) = &bench.saved {
                    ui.add_space(space::ROW);
                    theme::muted(ui, text::SAVED_TO.get(lang));
                    ui.add_space(2.0);
                    theme::path(ui, &to.to_string_lossy());
                }
            });
        });
    }
}

/// What was done and how, in one line: the changes first, the recipe after.
fn done_line(bench: &Bench, facts: &Facts, lang: Lang) -> String {
    // The shape is only worth stating when nothing above has already stated
    // it. Where there are findings they name the resolution and the frame rate
    // themselves, and repeating both underneath is a wrapped line spent saying
    // what the reader just read.
    let mut parts = diff_lines(bench, lang);
    if parts.is_empty() {
        parts.push(text::DIFF_NOTHING.get(lang).to_owned());
    }
    if let Some(done) = &bench.encoded {
        parts.push(
            text::RESULT_RECIPE
                .get(lang)
                .replace("{crf}", &done.crf.to_string())
                .replace("{encodes}", &done.encodes.max(1).to_string()),
        );
    }
    if bench.encoded.is_some() {
        parts.push(shape_only(facts));
    }
    parts.join("  ·  ")
}

/// A path shortened from the MIDDLE, because the ends are what identify it --
/// the drive it is on and the file it is.
fn elide_middle(path: &str, most: usize) -> String {
    let glyphs: Vec<char> = path.chars().collect();
    if glyphs.len() <= most {
        return path.to_owned();
    }
    let keep = most.saturating_sub(1) / 2;
    let head: String = glyphs[..keep].iter().collect();
    let tail: String = glyphs[glyphs.len() - keep..].iter().collect();
    format!("{head}…{tail}")
}

/// What the lossless pass actually did, in the charter's language.
fn diff_lines(bench: &Bench, lang: Lang) -> Vec<String> {
    bench
        .material
        .applied
        .iter()
        .map(|applied| match applied {
            Applied::AudioDropped => text::DIFF_AUDIO.get(lang).to_owned(),
            Applied::TailCut { was, now } => text::DIFF_TAIL
                .get(lang)
                .replace("{was}", &clock(*was))
                .replace("{now}", &clock(*now)),
        })
        .collect()
}

/// What is still wrong, and what the next pass will do about it.
fn pending_lines(pending: &[Pending], lang: Lang) -> Vec<(String, String)> {
    pending
        .iter()
        .map(|item| match item {
            Pending::Scale { from, to } => (
                text::FINDING_RESOLUTION
                    .get(lang)
                    .replace("{width}", &from.0.to_string())
                    .replace("{height}", &from.1.to_string()),
                text::FIX_RESOLUTION
                    .get(lang)
                    .replace("{width}", &to.0.to_string())
                    .replace("{height}", &to.1.to_string()),
            ),
            Pending::FrameRate { from, to } => (
                text::FINDING_FPS
                    .get(lang)
                    .replace("{fps}", &from.to_string())
                    .replace("{limit}", &to.to_string()),
                text::FIX_FPS.get(lang).replace("{limit}", &to.to_string()),
            ),
            Pending::Codec { from } => (
                text::FINDING_CODEC.get(lang).replace("{codec}", from),
                text::FIX_CODEC.get(lang).to_owned(),
            ),
            Pending::OpeningCut { seconds } => (
                text::TRIM_HEAD_WARNING
                    .get(lang)
                    .replace("{shift}", &clock(*seconds)),
                text::TRIM_HEAD_COSTS_REENCODE.get(lang).to_owned(),
            ),
            Pending::TooLarge { now, limit } => (
                text::FINDING_TOO_LARGE
                    .get(lang)
                    .replace("{size}", &now.div_ceil(1024 * 1024).to_string())
                    .replace("{limit}", &limit.div_ceil(1024 * 1024).to_string()),
                text::FIX_SIZE
                    .get(lang)
                    .replace("{limit}", &limit.div_ceil(1024 * 1024).to_string()),
            ),
        })
        .collect()
}

/// What the last Phase B run concluded. Amber for "nothing went wrong but you
/// should know"; red only for a genuine failure.
fn said_line(ui: &mut egui::Ui, lang: Lang, said: &Said) {
    match said {
        Said::MaterialWins { av1_was } => {
            ui.add(egui::Label::new(
                egui::RichText::new(text::SAID_MATERIAL_WINS.get(lang))
                    .size(theme::size::SMALL)
                    .color(theme::INK_MUTED),
            ));
            // The number it lost by. Press made a choice on the charter's
            // behalf; the least it can do is show its working.
            if let Some(was) = av1_was {
                ui.add_space(2.0);
                theme::muted(ui, &format!("AV1: {}", megabytes(*was)));
            }
        }
        Said::StillTooLarge { smallest } => {
            theme::heading(ui, text::STILL_TOO_LARGE.get(lang), theme::NOTICE);
            ui.add_space(space::TIGHT);
            ui.add(egui::Label::new(
                egui::RichText::new(
                    text::STILL_TOO_LARGE_DETAIL
                        .get(lang)
                        .replace("{size}", &smallest.div_ceil(1024 * 1024).to_string())
                        .replace(
                            "{limit}",
                            &limits::MAX_BYTES.div_ceil(1024 * 1024).to_string(),
                        ),
                )
                .size(theme::size::SMALL),
            ));
        }
        Said::Cancelled => theme::muted(ui, text::CANCELLED.get(lang)),
        Said::Failed(reason) => {
            theme::heading(ui, text::FAILED.get(lang), theme::ALARM);
            ui.add_space(space::TIGHT);
            ui.add(egui::Label::new(
                egui::RichText::new(reason).size(theme::size::SMALL),
            ));
        }
    }
}

/// How wide a centred block is allowed to be.
///
/// 520 in a 960 window was a narrow column with a lot of air either side, and
/// the narrowness cost lines: a finding worded in German wrapped to three where
/// it fits in two. 640 keeps prose near a comfortable measure and stops the
/// window looking like it is using a third of itself.
const CENTRED_W: f32 = 640.0;

/// Centres a block in whatever is left, at whatever height it turns out to be.
///
/// The height is not guessed: it is measured after drawing and remembered, so
/// the next frame centres the real thing. A guessed height is a card that is
/// either clipped or floating slightly off-centre, and neither is visible enough
/// to notice or invisible enough to ignore. The first frame after a change is
/// off by the difference and the repaint timer settles it before an eye could
/// follow.
fn centred(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
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
fn centred_row(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
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

/// The transport's height, reserved WHOLE and never varying.
///
/// This is the fix for the worst bug the layout has had: the trim warning used
/// to appear below the timeline when the opening handle left zero, and the
/// bottom-up layout pushed everything above it upwards -- so the control being
/// dragged MOVED under the pointer, mid-drag. Direct manipulation with a target
/// that relocates while you manipulate it is not a rough edge, it is the thing
/// not working.
///
/// The warning went to the readout, where consequences belong, and this block
/// became a constant: timeline, a row, a summary line, and the gaps between.
/// Nothing in the workbench changes height for any reason, so the preview never
/// resizes either and the picture does not jump when a handle is touched.
const TRANSPORT_H: f32 = crate::timeline::HEIGHT + space::TIGHT + ROW_H + space::TIGHT + LINE_H;
/// One line of small text, and one row of controls.
const LINE_H: f32 = 18.0;
const ROW_H: f32 = 34.0;

fn workbench_column(
    ui: &mut egui::Ui,
    lang: Lang,
    loaded: &mut Inspect,
    texture: Option<&egui::TextureHandle>,
) {
    let column = ui.max_rect();
    let controls = slice_bottom(column, TRANSPORT_H);
    let glass = egui::Rect::from_min_max(
        column.min,
        egui::pos2(column.right(), controls.top() - space::ROW),
    );

    region(ui, "the viewport", glass, |ui| {
        viewport(ui, texture, glass.size())
    });
    region(ui, "the transport", controls, |ui| {
        transport(ui, lang, loaded)
    });
}

/// A framed viewport with the picture letterboxed inside it.
///
/// The frame is the thing that fills the column; the video sits in the middle of
/// it at whatever size its own shape allows. That is the difference between a
/// deliberate viewport and leftover space: a 4:3 source in a 16:9 column leaves
/// bars either way, and bars inside a drawn frame read as a viewport while the
/// same bars against the page read as a layout that did not work out.
fn viewport(ui: &mut egui::Ui, texture: Option<&egui::TextureHandle>, room: egui::Vec2) {
    let (rect, _) = ui.allocate_exact_size(room, egui::Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(theme::RADIUS),
        theme::BACKDROP,
        egui::Stroke::new(1.0, theme::HAIRLINE),
        egui::StrokeKind::Inside,
    );

    let Some(texture) = texture else {
        return;
    };
    let size = texture.size();
    let (w, h) = (size[0] as f32, size[1] as f32);
    // One pixel of inset, so the picture never sits on the frame's own stroke.
    let inner = rect.shrink(1.0);
    let scale = (inner.width() / w).min(inner.height() / h);
    let drawn = egui::vec2(w * scale, h * scale);
    egui::Image::new(texture)
        .corner_radius(egui::CornerRadius::same(theme::RADIUS - 1))
        .paint_at(ui, egui::Rect::from_center_size(rect.center(), drawn));
}

/// Transport, timeline and what the trim currently means. Written bottom-up so
/// it can be dropped into a bottom-up layout unchanged.
fn transport(ui: &mut egui::Ui, lang: Lang, loaded: &mut Inspect) {
    // Destructured so the preview, the position and the trim are three separate
    // borrows: the transport reads one and writes the others in the same breath,
    // which through `loaded.` alone the compiler cannot allow.
    let Inspect {
        facts,
        preview,
        at,
        trim,
        ..
    } = loaded;
    let Some(facts) = facts.as_ref() else {
        return;
    };
    let duration = facts.duration.max(0.001);

    // A source's last frame sits one interval BEFORE its duration, so asking the
    // decoder for a picture at exactly `duration` asks for one that does not
    // exist and it returns nothing. The scrubber can be dragged to the very end,
    // so the position it asks to SEE is held just inside the material -- while
    // the position it reports, and the position the trim uses, are untouched.
    let inside = |at: f64| (at - 1.0 / 24.0).clamp(0.0, duration);

    // Top-down, in fixed slots. The timeline sits FIRST -- directly under the
    // picture it scrubs -- and everything after it is a readout of what the
    // handles currently say.
    // Touching the strip at all stops playback: the charter is aiming now, and a
    // picture that keeps moving under a handle being placed is aiming at a
    // moving target.
    let moved = match crate::timeline::timeline(ui, duration, *at, trim, true) {
        Change::None => None,
        Change::Seek(to) => Some(to),
        // Dragging a handle moves the picture AND the playhead to that handle,
        // because the frame being cut to is the entire reason to look at a
        // preview at all.
        Change::Trim { opening_moved } => Some(if opening_moved {
            trim.cut_opening
        } else {
            trim.keep_until.unwrap_or(duration)
        }),
    };
    if let Some(to) = moved {
        *at = to;
        if let Some(preview) = preview.as_ref() {
            preview.seek(inside(to));
        }
    }
    ui.add_space(space::TIGHT);

    // One row, one height, whatever is in it. The transport on the left, the way
    // back on the right.
    ui.allocate_ui(egui::vec2(ui.available_width(), ROW_H), |ui| {
        ui.horizontal_centered(|ui| {
            if let Some(preview) = preview.as_ref() {
                let playing = preview.playing();
                if theme::transport_button(ui, if playing { "⏸" } else { "▶" }).clicked() {
                    if playing {
                        preview.pause();
                    } else {
                        // Play always plays something. Sitting at the end of the
                        // kept region -- which is where the last playback left
                        // the head, and where dragging the end handle puts it --
                        // it rewinds to the start of that region instead of
                        // starting a decoder that immediately runs out.
                        let end = trim.keep_until.unwrap_or(duration);
                        let from = if *at >= end - 0.05 || *at < trim.cut_opening {
                            trim.cut_opening
                        } else {
                            *at
                        };
                        *at = from;
                        preview.play(from);
                    }
                }
                ui.add_space(space::ROW);
            }

            // Monospaced, so the digits do not change width as they count and
            // shove everything beside them left and right.
            theme::clock_pair(ui, &clock(*at), &clock(duration));

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if !trim.is_untouched() && theme::chip(ui, text::TRIM_RESET.get(lang)).clicked() {
                    *trim = Trim::default();
                }
            });
        });
    });
    ui.add_space(space::TIGHT);

    if trim.is_untouched() {
        theme::muted(ui, text::TRIM_NONE.get(lang));
    } else {
        let kept = trim.kept_seconds(duration);
        theme::muted(
            ui,
            &text::TRIM_TAIL_ONLY
                .get(lang)
                .replace("{kept}", &clock(kept))
                .replace("{cut}", &clock(duration - kept)),
        );
    }
}

/// The findings, and what is about to be done about each of them.
fn verdict_card(ui: &mut egui::Ui, lang: Lang, loaded: &Inspect, no_encoder: bool) {
    let opening = loaded.trim.cut_opening;
    // Sized to its contents, not stretched to the column.
    //
    // It used to fill the column, on the reasoning that "a card that stops
    // halfway down leaves a rectangle of nothing beside a rectangle of
    // something". That was wrong, and a screenshot settled it: a border drawn
    // around four hundred pixels of nothing is worse than the nothing.
    theme::card(ui, |ui| {
        ui.set_width(ui.available_width());
        // The WHOLE path, wrapped. A charter with three exports of the same
        // scene in three folders cannot tell them apart by file name, and the
        // one thing a formatter must never be vague about is which file it is
        // looking at.
        theme::path(ui, &loaded.path.to_string_lossy());
        ui.add_space(space::TIGHT);

        let Some(facts) = &loaded.facts else {
            theme::heading(ui, text::VERDICT_UNREADABLE.get(lang), theme::ALARM);
            return;
        };

        // SIZE IS NOT MENTIONED HERE, and its absence is the whole point of the
        // phase split. Whether this video fits in 80 MB is not a deterministic
        // question -- it depends on how compressible the footage turns out to
        // be, which nothing short of encoding it will say. Naming it on a screen
        // where every other line is settled would promise an answer Press does
        // not have yet.
        let deterministic = findings(facts, lang);
        if deterministic.is_empty() {
            theme::heading(ui, text::PHASE_A_CLEAR.get(lang), theme::GOOD);
            ui.add_space(space::TIGHT);
            theme::muted(ui, text::PHASE_A_CLEAR_HINT.get(lang));
        } else {
            theme::heading(ui, text::VERDICT_WILL_FIX.get(lang), theme::NOTICE);
        }
        ui.add_space(space::TIGHT);
        theme::muted(ui, &shape(facts, loaded.bytes));

        // Every finding, not only the first. The gate reports one violation
        // because it is refusing; Press is fixing, so it lists all of them --
        // being told about the audio and then, on the next attempt, about the
        // frame rate is the round trip this whole tool exists to remove.
        for (finding, fix) in deterministic {
            ui.add_space(space::ROW);
            ui.horizontal_top(|ui| {
                ui.label(egui::RichText::new("•").color(theme::NOTICE));
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(finding).color(theme::INK));
                    ui.add_space(2.0);
                    theme::muted(ui, &fix);
                });
            });
        }

        // The consequence of the one choice the charter makes. It lives HERE,
        // not under the timeline, and that is not cosmetic: below the timeline
        // it appeared when the opening handle left zero and pushed the whole
        // workbench upward, moving the control being dragged out from under the
        // pointer. Consequences belong where consequences are stated.
        if opening > 0.0 {
            ui.add_space(space::BLOCK);
            ui.label(
                egui::RichText::new(
                    text::TRIM_HEAD_WARNING
                        .get(lang)
                        .replace("{shift}", &clock(opening)),
                )
                .size(theme::size::SMALL)
                .color(theme::NOTICE),
            );
            ui.add_space(space::TIGHT);
            theme::muted(ui, text::TRIM_HEAD_COSTS_REENCODE.get(lang));
        }

        // Inside the card, not floating beneath it: it is part of the verdict --
        // "and by the way nothing can be done about any of this" -- and a line
        // outside the panel was a line the column had no room reserved for.
        if no_encoder {
            ui.add_space(space::BLOCK);
            ui.label(
                egui::RichText::new(text::NO_ENCODER.get(lang))
                    .size(theme::size::SMALL)
                    .color(theme::ALARM),
            );
        }
    });
}

/// The DETERMINISTIC findings: everything whose answer follows from the file.
///
/// Size is deliberately absent. It is the one gate rule that is not settled by
/// looking at the header, and it belongs to Phase B along with the search that
/// resolves it.
fn findings(facts: &Facts, lang: Lang) -> Vec<(String, String)> {
    let mut out = Vec::new();

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

fn shape(facts: &Facts, bytes: u64) -> String {
    format!("{}  ·  {}", shape_only(facts), megabytes(bytes))
}

/// The same, without the weight -- for the places that state the weight
/// separately and larger.
fn shape_only(facts: &Facts) -> String {
    format!(
        "{}  ·  {}×{}  ·  {:.0} fps  ·  {}",
        facts.codec.unwrap_or(if facts.raw_fourcc.is_empty() {
            "?"
        } else {
            &facts.raw_fourcc
        }),
        facts.width,
        facts.height,
        facts.fps,
        clock(facts.duration),
    )
}

fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    format!("{}:{:04.1}", (seconds / 60.0) as u64, seconds % 60.0)
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

/// The inspect screen lists the deterministic problems; the gate reports the
/// first problem of any kind. They must not contradict each other about the
/// ones they both cover -- a file the gate refuses for its frame rate with
/// nothing listed under it is a window disagreeing with the thing it exists to
/// report.
///
/// Size is excluded on both sides, because it is exactly what the inspect screen
/// declines to have an opinion about.
#[allow(dead_code)]
fn verdict_matches_findings(facts: &Facts, lang: Lang) -> bool {
    // Under the limit by construction, so `check` can only complain about the
    // deterministic rules.
    let refused = !gate::check(0, Some(facts)).passes();
    refused == !findings(facts, lang).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_fixture(mut check: impl FnMut(&Facts, u64, &Path)) {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut seen = 0;
        for entry in std::fs::read_dir(dir).expect("fixtures") {
            let path = entry.expect("entry").path();
            if path.extension().is_none_or(|e| e != "mp4") {
                continue;
            }
            let bytes = std::fs::read(&path).expect("fixture");
            let Some(facts) = gate::probe(&bytes) else {
                continue;
            };
            check(&facts, bytes.len() as u64, &path);
            seen += 1;
        }
        assert!(seen > 10, "expected the fixture set, found {seen}");
    }

    /// The screen lists every finding while the gate reports only the first.
    /// They still have to agree on WHETHER anything is wrong -- a file the gate
    /// refuses with nothing listed, or a file it accepts with a bullet under it,
    /// is a window that contradicts the thing it exists to report.
    #[test]
    fn the_screen_and_the_gate_never_disagree_about_whether_it_passes() {
        every_fixture(|facts, _bytes, path| {
            for lang in Lang::ALL {
                assert!(
                    verdict_matches_findings(facts, lang),
                    "{} disagrees with the gate in {}",
                    path.display(),
                    lang.endonym()
                );
            }
        });
    }

    /// A finding that renders with its braces still in it is a placeholder whose
    /// name was changed on one side only.
    #[test]
    fn no_finding_renders_with_an_unfilled_placeholder() {
        every_fixture(|facts, _bytes, path| {
            for lang in Lang::ALL {
                for (finding, fix) in findings(facts, lang) {
                    for rendered in [&finding, &fix] {
                        assert!(
                            !rendered.contains('{') && !rendered.contains('}'),
                            "{}: {rendered}",
                            path.display()
                        );
                    }
                }
            }
        });
    }

    // ── every stage fits the canvas ──────────────────────────────────────
    //
    // `region` asserts in debug that what was laid out actually fitted, which
    // makes it a very good test -- and it had never once run against Working or
    // Finished, because reaching those states needs an encoder and a real file.
    // So the action bar's height was a guess nobody had checked, it was two
    // pixels short, and the buttons hung out of the bottom of the window.
    //
    // In-module rather than under `tests/`, because the stages are private and
    // the alternative is a public constructor that exists only for a test --
    // scaffolding in the product to make the product testable.

    fn window_input() -> egui::RawInput {
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::pos2(0.0, 0.0),
                egui::Vec2::from(WINDOW),
            )),
            ..Default::default()
        }
    }

    /// Runs `frames` frames of the real window and returns what was painted last.
    fn run(press: &mut Press, ctx: &egui::Context, frames: usize) -> egui::Rect {
        let mut painted = egui::Rect::NOTHING;
        for _ in 0..frames {
            let mut frame = eframe::Frame::_new_kittest();
            let mut output = ctx.run_ui(window_input(), |ui| {
                use eframe::App;
                press.ui(ui, &mut frame);
            });
            output.textures_delta.clear();
            painted = egui::Rect::NOTHING;
            for shape in &output.shapes {
                let bounds = shape
                    .shape
                    .visual_bounding_rect()
                    .intersect(shape.clip_rect);
                if bounds.is_positive() {
                    painted = painted.union(bounds);
                }
            }
        }
        painted
    }

    const LONG: &str = "D:\\User\\Videos\\Others\\少女舞乐哭Live-最终版-导出-4k-2026.mp4";

    fn facts_of(codec: &'static str, w: u32, h: u32, fps: f64) -> Facts {
        Facts {
            width: w,
            height: h,
            fps,
            duration: 110.6,
            codec: Some(codec),
            raw_fourcc: codec.to_owned(),
            has_audio: false,
        }
    }

    fn a_bench(applied: Vec<Applied>, encoded: Option<Box<Encoded>>, said: Option<Said>) -> Bench {
        Bench {
            source: PathBuf::from(LONG),
            source_bytes: 98_000_000,
            source_facts: facts_of("h264", 3840, 2160, 120.0),
            opening_cut: 1.5,
            material: Box::new(Material::for_test(
                PathBuf::from(LONG),
                96_000_000,
                facts_of("h264", 3840, 2160, 120.0),
                applied,
            )),
            encoded,
            said,
            saved: None,
            work: Arc::new(Workspace::open().expect("a workspace")),
        }
    }

    /// Every screen, in every language, laid out on the one canvas there is.
    ///
    /// The debug assertion inside `region` is what does the judging; this only
    /// has to put each screen on the glass. A long CJK path is used
    /// deliberately -- it is both the widest text and the tallest when wrapped.
    #[test]
    fn every_stage_fits_the_window() {
        let sample = crate::ffmpeg::Progress {
            out_time: 41.0,
            fraction: Some(0.46),
            bytes: 84_500_000,
            speed: Some(1.8),
        };

        for lang in Lang::ALL {
            let ctx = egui::Context::default();
            let mut press = Press::new(&ctx, lang, None);

            // Empty.
            run(&mut press, &ctx, 2);

            // Both phases running.
            let phases = [
                Phase::A(phase_a::Step::Copying(Some(sample))),
                Phase::A(phase_a::Step::Verifying),
                Phase::B(phase_b::Step::Encoding {
                    crf: 26,
                    attempt: 2,
                    progress: Some(sample),
                }),
                Phase::B(phase_b::Step::Recalibrating { from: 32, to: 20 }),
            ];
            for phase in phases {
                let (_tx, inbox) = channel();
                press.stage = Stage::Working(Box::new(Working {
                    source: PathBuf::from(LONG),
                    phase,
                    cancel: Cancel::new(),
                    inbox,
                    work: Arc::new(Workspace::open().expect("a workspace")),
                    held: Held {
                        source_bytes: 98_000_000,
                        source_facts: facts_of("h264", 3840, 2160, 120.0),
                        trim: Trim::default(),
                        material: None,
                        encoded: None,
                    },
                }));
                run(&mut press, &ctx, 2);
            }

            // The bench, in every state it can be in: nothing done yet, a
            // result in hand, and each way a run can end without one.
            let benches = [
                a_bench(vec![], None, None),
                a_bench(
                    vec![
                        Applied::AudioDropped,
                        Applied::TailCut {
                            was: 110.6,
                            now: 92.0,
                        },
                    ],
                    Some(Box::new(Encoded {
                        path: PathBuf::from(LONG),
                        bytes: 38_200_000,
                        facts: facts_of("av1", 2560, 1440, 60.0),
                        crf: 20,
                        encodes: 2,
                    })),
                    None,
                ),
                // Ready AND already written: the fullest the action bar ever
                // gets, which is the row that has to be measured.
                Bench {
                    saved: Some(PathBuf::from(LONG)),
                    ..a_bench(
                        vec![Applied::AudioDropped],
                        Some(Box::new(Encoded {
                            path: PathBuf::from(LONG),
                            bytes: 38_200_000,
                            facts: facts_of("av1", 2560, 1440, 60.0),
                            crf: 20,
                            encodes: 2,
                        })),
                        None,
                    )
                },
                // EVERY problem at once, which is the tallest the card can be:
                // a codec nothing plays, over the pixel budget, over the frame
                // rate, an opening cut pending, and over the limit -- five
                // findings, each two lines, in the language that words them
                // longest.
                Bench {
                    source_facts: facts_of("vp09", 3840, 2160, 120.0),
                    material: Box::new(Material::for_test(
                        PathBuf::from(LONG),
                        396_000_000,
                        facts_of("vp09", 3840, 2160, 120.0),
                        vec![
                            Applied::AudioDropped,
                            Applied::TailCut {
                                was: 110.6,
                                now: 92.0,
                            },
                        ],
                    )),
                    ..a_bench(vec![], None, None)
                },
                a_bench(
                    vec![Applied::AudioDropped],
                    None,
                    Some(Said::MaterialWins {
                        av1_was: Some(112_000_000),
                    }),
                ),
                a_bench(
                    vec![Applied::AudioDropped],
                    None,
                    Some(Said::StillTooLarge {
                        smallest: 91_000_000,
                    }),
                ),
                a_bench(vec![Applied::AudioDropped], None, Some(Said::Cancelled)),
                a_bench(
                    vec![Applied::AudioDropped],
                    None,
                    Some(Said::Failed(
                        "the encode failed: ffmpeg exited with status 1".to_owned(),
                    )),
                ),
            ];
            for bench in benches {
                press.stage = Stage::Bench(Box::new(bench));
                // Three frames: the centred row measures itself and settles on
                // the second, and the third is the one that has to be right.
                let painted = run(&mut press, &ctx, 3);

                let screen =
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::Vec2::from(WINDOW));
                assert!(painted.is_positive(), "nothing was painted at all");
                assert!(
                    screen.contains_rect(painted.shrink(0.5)),
                    "{}: painted {painted:?} outside a window of {screen:?}",
                    lang.endonym()
                );
            }
        }
    }
}
