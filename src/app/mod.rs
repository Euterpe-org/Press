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
//!
//! # What lives beside this file
//!
//! Two things were pulled out, and both for the same reason: they are the parts
//! that can be judged WITHOUT running a window, and they were the parts hardest
//! to look at while they were mixed in with the painting.
//!
//! * [`say`] -- every sentence a screen prints. Pure: a bench and a language in,
//!   the exact string out. This is where every wording bug this project has had
//!   actually lived, and where the test that reads them had grown a hand-copy of
//!   the logic because it could not reach the real thing without a `Ui`.
//! * [`layout`] -- cutting regions out of a canvas whose size is known in
//!   advance, and asserting that what a screen laid out fits the space it was
//!   handed.
//!
//! What is left here is the STATE MACHINE -- the stages, the transitions between
//! them, and the four screens that draw them.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use eframe::egui;

mod layout;
mod say;

use layout::*;
use say::*;

use crate::encoder::Absent;
use crate::ffmpeg::{Cancel, Ffmpeg};
use crate::gate::{self, Facts, limits};
use crate::i18n::{Lang, Text, text};
use crate::job::{Job, Request};
use crate::phase_a::{self, Applied, Material};
use crate::phase_b::{self, Aim, Encoded, Pending};
use crate::plan::Trim;
use crate::preview::Preview;
use crate::theme::space;
use crate::timeline::Change;
use crate::workspace::Workspace;
use crate::{crop, fonts, theme};

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
    /// The encoder, or WHY there is none. Carried as the reason rather than as
    /// an absence: the two causes ask completely different things of the person
    /// reading about them, and collapsing them told somebody whose antivirus
    /// had eaten the unpacked file that their download was broken.
    ffmpeg: Result<Ffmpeg, Absent>,
    stage: Stage,
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
    job: Job,
    /// Where the scrubber is, in seconds.
    at: f64,
    /// `None` when there is no encoder to decode with. The verdict does not need
    /// one -- the gate reads the header -- so the screen still works without it.
    view: Option<Viewfinder>,
}

/// The decoder, and the frame it last handed over.
///
/// One value because the second is meaningless without the first. They were two
/// independent `Option`s on `Press`, which made "a texture with no decoder"
/// representable, and made every stage transition responsible for remembering to
/// clear both. Born and dropped with the screen that looks through it, so there
/// is nothing left to remember.
struct Viewfinder {
    preview: Preview,
    texture: Option<egui::TextureHandle>,
    /// The stamp of the frame in `texture`, so an unchanged frame is not
    /// re-uploaded on every repaint.
    shown: f64,
}

impl Viewfinder {
    fn open(ffmpeg: &Ffmpeg, source: &Path, facts: &Facts) -> Self {
        // The frame ffmpeg will DECODE, not the one `stsd` describes. The
        // viewfinder scales into a box of exactly this shape, so on a rotated
        // clip the coded size squeezed an upright picture into a landscape box
        // -- with the crop rectangle drawn on top of it, aimed at an axis that
        // was not there.
        let (w, h) = facts.picture();
        let preview = Preview::open(ffmpeg.binary(), source, w, h, facts.fps);
        // Something on screen immediately: an empty frame under a verdict reads
        // as a decoder that failed.
        preview.seek(0.0);
        Self {
            preview,
            texture: None,
            shown: f64::NAN,
        }
    }

    /// Put the decoder's newest frame on the GPU, if it is newer than the last.
    fn refresh(&mut self, ctx: &egui::Context) {
        let Some(frame) = self.preview.frame() else {
            return;
        };
        if frame.pts == self.shown && self.texture.is_some() {
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
        self.shown = frame.pts;
    }
}

struct Working {
    job: Job,
    phase: Phase,
    cancel: Cancel,
    inbox: Receiver<Note>,
    /// Held so the temp directory outlives the worker that is writing into it.
    work: Arc<Workspace>,
}

/// Which phase is running, and what it already has in hand.
///
/// Phase A has nothing yet, by definition -- it is the thing that produces the
/// material. Phase B cannot start without one. Carrying the material on the B
/// arm is what makes that a fact about the type rather than an `Option` the
/// screen has to unwrap and an "unreachable" error path to write for it.
enum Phase {
    A(phase_a::Step),
    B {
        /// Phase A's output. **Every** Phase B run encodes from this and never
        /// from a previous Phase B result -- that would be a second generation
        /// of loss for no gain.
        material: Box<Material>,
        /// The best result so far, kept across a run that does worse.
        best: Option<Box<Encoded>>,
        step: phase_b::Step,
    },
}

/// Between the phases: what we have, and what is still wrong with it.
///
/// This is the hub. Phase B returns here rather than to a terminal screen,
/// which is what makes "try to make it smaller" an ordinary press of the same
/// button rather than a mode of its own.
struct Bench {
    job: Job,
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
///
/// EVERY variant is rendered. A run that comes back to a card identical to the
/// one it left reads as a run that never happened, and the charter waited
/// minutes for it.
enum Said {
    /// AV1 was tried and came back bigger. The lossless material ships.
    /// `av1_was` is `None` when the encode was killed on its way past the
    /// material's weight rather than allowed to finish, which is the ordinary
    /// case.
    MaterialWins {
        av1_was: Option<u64>,
    },
    /// The crop or the opening cut could not be carried out at any quality
    /// without the file gaining weight, so it was not carried out.
    RequestDropped,
    /// Not even the coarsest quality fits. The remaining lever is length.
    StillTooLarge {
        smallest: u64,
    },
    Cancelled,
    Failed(String),
}

impl Said {
    /// Whether pressing the same button again would reach the same answer.
    ///
    /// Phase B is deterministic in the material, the aim and the request, and
    /// none of those change by waiting. Every one of these verdicts is
    /// therefore final for as long as the bench stands -- offering the run
    /// again just spends the same minutes arriving at the same screen.
    fn settled(&self) -> bool {
        matches!(
            self,
            Self::MaterialWins { .. } | Self::RequestDropped | Self::StillTooLarge { .. }
        )
    }
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

    /// What the charter asked for that has not been carried out yet.
    ///
    /// ONE question, asked once. A single ffmpeg invocation applies the whole
    /// request at once, so "has the crop happened" and "has the opening moved"
    /// cannot have different answers -- and asking them separately, as two
    /// members that each re-derived it, only made two places to get it wrong.
    fn outstanding(&self) -> Request {
        if self.encoded.is_some() {
            return Request::default();
        }
        self.job.request(self.material.facts.picture())
    }

    /// Everything the next encode would do. Not all of it is a fault.
    fn pending(&self) -> Vec<Pending> {
        let (_, bytes, facts) = self.have();
        phase_b::pending(facts, bytes, &self.outstanding())
    }

    /// Whether anything stands between this file and an upload.
    ///
    /// ASKED OF THE FILE IN HAND, not of the plan for the next encode. Those are
    /// different questions and the answer used to come from the wrong one:
    /// `pending()` describes what the encode would do AFTER the crop, so a crop
    /// that would bring an over-sized picture under the budget erased the fault
    /// from the list -- and the card then offered Save on a file the server was
    /// always going to refuse, because the crop had not happened yet.
    ///
    /// A crop and an opening cut never appear here at all. The file is
    /// uploadable without them and merely is not yet what was asked for.
    fn blocked(&self) -> bool {
        let (_, bytes, facts) = self.have();
        !crate::gate::check(bytes, Some(facts)).passes()
    }

    /// Whether pressing the button again could change anything.
    ///
    /// Once Phase B has handed back a file, Press has given its answer -- there
    /// is no second opinion to ask for, because the quality was not a guess.
    /// And once it has given a verdict WITHOUT a file, that verdict is just as
    /// final: the run is deterministic in the material, the aim and the request,
    /// so a second press spends the same minutes reaching the same screen.
    fn worth_running(&self) -> Option<Aim> {
        if self.encoded.is_some() || self.said.as_ref().is_some_and(Said::settled) {
            return None;
        }
        phase_b::aim(
            &self.material.facts,
            self.material.bytes,
            &self.outstanding(),
        )
    }
}

/// What a worker thread sends back.
///
/// A STEP, not a whole `Phase`. The worker knows how far its own run has got and
/// nothing else -- it does not hold the material, it has never heard of the
/// screen, and letting it post a complete phase meant it had to name state that
/// belongs to the window.
enum Note {
    StepA(phase_a::Step),
    StepB(phase_b::Step),
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
    pub fn new(ctx: &egui::Context, lang: Lang, ffmpeg: Result<Ffmpeg, Absent>) -> Self {
        fonts::install(ctx, lang);
        theme::install(ctx);
        Self {
            lang,
            ffmpeg,
            stage: Stage::Empty,
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

/// What the title bar says.
///
/// The version belongs HERE rather than inside the window. It is never what
/// anybody came for, but it is the first thing asked for when a charter reports
/// that something misbehaved -- and the title bar is the one part of a window
/// that survives being screenshotted, cropped, described over a phone call, or
/// read off a taskbar preview. It used to sit in the header instead, in small
/// grey text beside the language picker, competing for the one row that is
/// about the file.
pub fn window_title(lang: Lang) -> String {
    format!("{} {VERSION}", text::WINDOW_TITLE.get(lang))
}

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

impl Press {
    fn action_height(&self) -> f32 {
        match &self.stage {
            Stage::Empty => 0.0,
            Stage::Inspect(_) => {
                if self.ffmpeg.is_ok() {
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
            Stage::Inspect(_) => {
                if self.ffmpeg.is_ok() {
                    ui.vertical_centered(|ui| {
                        if theme::primary(ui, text::NEXT_STEP.get(lang)).clicked() {
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
                let ready = !bench.blocked();
                let has_work = !bench.pending().is_empty();
                let runnable = bench.worth_running();
                // The card says to trim it shorter; without this there is
                // nowhere to do that but drag the file in again.
                let shorter_only = matches!(bench.said, Some(Said::StillTooLarge { .. }));
                let saved = bench.saved.clone();
                let mut pressed = None;

                // A row, centred properly. `vertical_centered` centres a single
                // widget but not a `horizontal` inside it -- the row claims the
                // full width and its contents start hard against the left edge.
                centred_row(ui, "bench-actions", |ui| {
                    // ONE ACTION, TWO MEANINGS. It clears the workspace and
                    // goes back to the drop screen either way -- but before the
                    // file is written that is throwing work away, and after it
                    // is written the work is safely on disk and this only starts
                    // the next video. Calling it "discard" once the file exists
                    // reads as though it might undo the save.
                    let leave = if saved.is_some() {
                        &text::ANOTHER
                    } else {
                        &text::ABANDON
                    };
                    if theme::quiet(ui, leave.get(lang)).clicked() {
                        pressed = Some(Act::Abandon);
                    }

                    // Only where it is actionable: the message above says to
                    // trim it shorter, and without this there is nowhere to do
                    // that but drag the file in again.
                    if shorter_only && theme::quiet(ui, text::BACK_TO_TRIM.get(lang)).clicked() {
                        pressed = Some(Act::BackToTrim);
                    }

                    // THE LOUD BUTTON IS ALWAYS THE NEXT USEFUL THING, and it
                    // MOVES as the run progresses -- which is itself a signal
                    // that something happened.
                    //
                    //   not ready       start
                    //   ready           make it smaller   (save is the quiet one)
                    //   encoded         save
                    //   saved           show in folder
                    //
                    // It used to be pinned to Save, which broke the rule twice
                    // over: on a file that was already fine the recommended
                    // action was a grey pill beside a big blue button, and after
                    // the file had been written the big blue button still said
                    // "save" -- pointing at the step just completed.
                    if saved.is_some() {
                        // Written. Uploading is what happens next and it starts
                        // in a file manager. Saving again is not offered: the
                        // file is on disk, the folder is one press away, and a
                        // third button here is a row with no focus.
                        if theme::primary(ui, text::SHOW_IN_FOLDER.get(lang)).clicked() {
                            pressed = Some(Act::Reveal);
                        }
                        return;
                    }

                    // `worth_running` is the only judge of this now. It refuses
                    // once a run has produced a file AND once a run has produced
                    // a verdict, because Phase B is deterministic in everything
                    // the bench holds -- pressing again spends the same minutes
                    // reaching the same screen. The action bar used to make that
                    // call itself, and made it only for one verdict out of four.
                    let offer_run = runnable.is_some();
                    if offer_run {
                        // "Make it smaller" only when that is genuinely all
                        // there is left to do. With a crop or an opening cut
                        // outstanding there is real work queued, and calling it
                        // "make it smaller" describes none of it.
                        let label = if has_work {
                            text::START.get(lang)
                        } else {
                            text::SHRINK_FURTHER.get(lang)
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
                });

                match pressed {
                    // Dropping the stage drops the Arc, and the last Arc takes
                    // the temp directory with it -- along with the decoder and
                    // its texture, which now live on the screen that used them
                    // rather than on the window that had to remember to clear
                    // them at every exit.
                    Some(Act::Abandon) => self.stage = Stage::Empty,
                    Some(Act::BackToTrim) => {
                        if let Stage::Bench(bench) =
                            std::mem::replace(&mut self.stage, Stage::Empty)
                        {
                            self.reopen(bench.job);
                        }
                    }
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
                                // The title bar carries the version, and the
                                // system will not re-read it on its own.
                                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(
                                    window_title(lang),
                                ));
                                fonts::install(ui.ctx(), lang);
                            }
                        }
                    });
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
        // TWO WAYS A FILE CAN FAIL AT THE DOOR, and they both end here rather
        // than on a screen built around a file nothing is known about. The old
        // shape kept an unreadable one in `Inspect` with `facts: None`, which
        // bought a workbench with no picture in it, a scrubber of zero length,
        // a verdict card reading "cannot read this", and no button -- a dead end
        // dressed as a workspace. Every screen past this point now has real
        // facts by construction, and the two `is_some()` guards, the `(0, 0)`
        // geometry fallback and the unreadable branch on the card are all gone
        // with the state that needed them.
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                // A real dead end -- a file another program holds open, a
                // disconnected drive. Said, not swallowed: resetting to the drop
                // screen with nothing written on it is indistinguishable from a
                // drop that was never noticed.
                self.give_up(&format!("{}: {error}", path.display()));
                return;
            }
        };
        let source_bytes = bytes.len() as u64;
        let facts = gate::judge(&bytes).0;
        drop(bytes);
        let Some(facts) = facts else {
            self.give_up(text::VERDICT_UNREADABLE.get(self.lang));
            return;
        };

        self.stage = Stage::Inspect(Box::new(Inspect {
            view: self
                .ffmpeg
                .as_ref()
                .ok()
                .map(|ffmpeg| Viewfinder::open(ffmpeg, path, &facts)),
            job: Job {
                source: path.to_path_buf(),
                bytes: source_bytes,
                facts,
                trim: Trim::default(),
                crop: None,
            },
            at: 0.0,
        }));
    }

    /// Drain the worker and keep the window repainting while anything moves.
    fn pump(&mut self, ctx: &egui::Context) {
        if let Stage::Inspect(loaded) = &mut self.stage
            && let Some(view) = &loaded.view
        {
            let preview = &view.preview;
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
                if let Some(end) = loaded.job.trim.keep_until
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
                Ok(Note::StepA(step)) => {
                    if let Phase::A(at) = &mut working.phase {
                        *at = step;
                    }
                }
                Ok(Note::StepB(step)) => {
                    if let Phase::B { step: at, .. } = &mut working.phase {
                        *at = step;
                    }
                }
                Ok(Note::SettledA(outcome)) => {
                    self.finish_phase_a(*outcome);
                    return;
                }
                Ok(Note::FittedB(outcome)) => {
                    self.finish_phase_b(*outcome);
                    return;
                }
                // The worker vanished without a verdict. The one way that
                // happens is a panic on its thread, which leaves its reason with
                // `bug` on the way out -- so the charter is told what actually
                // broke instead of that something did. Without it this said
                // "the worker stopped unexpectedly" and the real message went to
                // a console a release build does not have.
                Err(TryRecvError::Disconnected) => {
                    let reason = crate::bug::take()
                        .unwrap_or_else(|| "the worker stopped unexpectedly".to_owned());
                    self.give_up(&reason);
                    return;
                }
                Err(TryRecvError::Empty) => return,
            }
        }
    }

    /// Phase A: the deterministic pass. Nothing is asked of the charter here --
    /// every answer follows from the file.
    fn start_phase_a(&mut self) {
        let (Stage::Inspect(inspect), Ok(ffmpeg)) = (&self.stage, self.ffmpeg.clone()) else {
            return;
        };
        let job = inspect.job.clone();

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
            let work = Arc::clone(&work);
            let job = job.clone();
            move || {
                let tx2 = tx.clone();
                let outcome = phase_a::run(&ffmpeg, &job, &work, &cancel, |step| {
                    let _ = tx2.send(Note::StepA(step));
                });
                let _ = tx.send(Note::SettledA(Box::new(outcome)));
            }
        });

        self.stage = Stage::Working(Box::new(Working {
            job,
            phase: Phase::A(phase_a::Step::Reading),
            cancel,
            inbox,
            work,
        }));
    }

    fn finish_phase_a(&mut self, outcome: phase_a::Outcome) {
        match outcome {
            phase_a::Outcome::Settled(material) => self.show_bench(material, None, None),
            // Nothing was produced and nothing was lost, so the charter goes
            // back to exactly where they pressed the button -- trim included.
            phase_a::Outcome::Cancelled => {
                if let Stage::Working(working) = std::mem::replace(&mut self.stage, Stage::Empty) {
                    self.reopen(working.job);
                }
            }
            phase_a::Outcome::Failed(reason) => self.give_up(&reason),
        }
    }

    /// Phase B: the fight with 80 MB. `aim` is what the bench decided it is for.
    fn start_phase_b(&mut self, aim: Aim) {
        let Ok(ffmpeg) = self.ffmpeg.clone() else {
            return;
        };
        // TAKEN, and put back if it was not the screen this belongs to. The old
        // shape tested the stage, moved it out, and then asserted through an
        // `unreachable!` that it was still what the test had just said -- a
        // panic standing in for a fact the compiler was never told. One match is
        // the whole test now, and the failing arm costs the charter nothing.
        let bench = match std::mem::replace(&mut self.stage, Stage::Empty) {
            Stage::Bench(bench) => *bench,
            other => {
                self.stage = other;
                return;
            }
        };
        let work = Arc::clone(&bench.work);
        let asked = bench.outstanding();

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
                let outcome =
                    phase_b::run(&ffmpeg, &material, aim, &asked, &work, &cancel, |step| {
                        let _ = tx2.send(Note::StepB(step));
                    });
                let _ = tx.send(Note::FittedB(Box::new(outcome)));
            }
        });

        self.stage = Stage::Working(Box::new(Working {
            job: bench.job,
            phase: Phase::B {
                material: bench.material,
                best: bench.encoded,
                step: phase_b::Step::Encoding {
                    crf: crate::plan::CRF,
                    attempt: 1,
                    progress: None,
                },
            },
            cancel,
            inbox,
            work,
        }));
    }

    fn finish_phase_b(&mut self, outcome: phase_b::Outcome) {
        let Stage::Working(working) = &mut self.stage else {
            return;
        };
        let Phase::B { material, best, .. } = &mut working.phase else {
            return;
        };
        let material = material.clone();
        // A better file supersedes whatever came before it, and the workspace
        // takes the loser with it.
        let (encoded, said) = match outcome {
            phase_b::Outcome::Encoded(done) => (Some(done), None),
            phase_b::Outcome::MaterialWins { av1_was } => {
                (best.take(), Some(Said::MaterialWins { av1_was }))
            }
            phase_b::Outcome::RequestDropped => (best.take(), Some(Said::RequestDropped)),
            phase_b::Outcome::StillTooLarge { smallest } => {
                (best.take(), Some(Said::StillTooLarge { smallest }))
            }
            phase_b::Outcome::Cancelled => (best.take(), Some(Said::Cancelled)),
            phase_b::Outcome::Failed(reason) => (best.take(), Some(Said::Failed(reason))),
        };
        self.show_bench(material, encoded, said);
    }

    /// Rebuild the bench from the run that just finished.
    ///
    /// The material is an ARGUMENT rather than something fished back out of the
    /// stage, because both callers already hold one: Phase A has just produced
    /// it and Phase B has been carrying it all along. Reaching for it through an
    /// `Option` here bought an "only reachable if Phase A reported success
    /// without a material, which it cannot" branch -- a dead end written for a
    /// state the types now refuse to represent.
    fn show_bench(
        &mut self,
        material: Box<Material>,
        encoded: Option<Box<Encoded>>,
        said: Option<Said>,
    ) {
        let Stage::Working(working) = std::mem::replace(&mut self.stage, Stage::Empty) else {
            return;
        };
        let working = *working;
        self.stage = Stage::Bench(Box::new(Bench {
            job: working.job,
            material,
            encoded,
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
            .set_directory(bench.job.source.parent().unwrap_or(Path::new(".")))
            .set_file_name(crate::job::result_name(&bench.job.source))
            .add_filter("MP4", &["mp4"])
            .save_file()
        else {
            return;
        };
        // Already there. Copying a file onto itself truncates it, so the copy is
        // skipped -- but the SAVE is not: the charter picked a destination and
        // the file they wanted is sitting at it. Returning quietly left the card
        // still offering "save", which reads as a save that failed.
        if same_file(&from, &to) {
            if let Stage::Bench(bench) = &mut self.stage {
                bench.saved = Some(to);
            }
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
    }

    /// Back to the scrubber, with everything the charter decided still on it.
    ///
    /// Reached from a cancelled run, and from "even at the lowest quality it
    /// will not fit -- make it shorter", where the trim is the entire point of
    /// arriving. It used to take the job apart into five arguments and put it
    /// back together, and the tail of the trim did not survive the trip: the one
    /// screen a charter is sent to in order to SHORTEN their video opened with
    /// the whole source back in it.
    fn reopen(&mut self, job: Job) {
        self.stage = Stage::Inspect(Box::new(Inspect {
            view: self
                .ffmpeg
                .as_ref()
                .ok()
                .map(|ffmpeg| Viewfinder::open(ffmpeg, &job.source, &job.facts)),
            job,
            at: 0.0,
        }));
    }
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

impl Press {
    fn inspect(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let lang = self.lang;
        let missing = self.ffmpeg.as_ref().err().cloned();
        let Stage::Inspect(loaded) = &mut self.stage else {
            return;
        };
        if let Some(view) = &mut loaded.view {
            view.refresh(ctx);
        }

        let body = ui.max_rect();
        let workbench = egui::Rect::from_min_max(
            body.min,
            egui::pos2(body.left() + WORKBENCH_W, body.bottom()),
        );
        let readout =
            egui::Rect::from_min_max(egui::pos2(body.right() - READOUT_W, body.top()), body.max);

        region(ui, "the workbench", workbench, |ui| {
            workbench_column(ui, lang, loaded);
        });
        let mut picked = None;
        region(ui, "the readout", readout, |ui| {
            picked = verdict_card(ui, lang, loaded, missing.as_ref());
        });
        if let Some(choice) = picked {
            loaded.job.crop = choice;
        }
    }

    /// Centred in the body: there is nothing else to look at, and a progress bar
    /// pinned to the top leaves the rest of the window empty for no reason.
    fn working(&mut self, ui: &mut egui::Ui) {
        let lang = self.lang;
        let Stage::Working(working) = &self.stage else {
            return;
        };

        // A run has three things worth saying: WHAT is happening, HOW FAR it has
        // got, and HOW BIG the result is getting. `detail` carries the last of
        // those plus the quality being used -- which is the whole reason a
        // second attempt exists.
        let (label, fraction, detail) = match &working.phase {
            Phase::A(phase_a::Step::Reading) => {
                (text::READING.get(lang).to_owned(), None, Vec::new())
            }
            Phase::A(phase_a::Step::Copying(p)) => (
                text::STEP_COPYING.get(lang).to_owned(),
                p.and_then(|p| p.fraction),
                written_line(lang, p.map(|p| p.bytes)),
            ),
            Phase::A(phase_a::Step::Verifying)
            | Phase::B {
                step: phase_b::Step::Verifying,
                ..
            } => (text::STEP_VERIFYING.get(lang).to_owned(), None, Vec::new()),
            Phase::B {
                step:
                    phase_b::Step::Encoding {
                        crf,
                        attempt,
                        progress,
                    },
                ..
            } => {
                // THE ATTEMPT NUMBER, from the second one onward. Without it a
                // search that takes three encodes fills the bar, resets it, and
                // shows the identical sentence again -- and from the outside
                // that is indistinguishable from a loop.
                let mut label = text::STEP_ENCODING.get(lang).to_owned();
                if *attempt > 1 {
                    label = format!(
                        "{label}  {}",
                        text::STEP_ATTEMPT
                            .get(lang)
                            .replace("{n}", &attempt.to_string())
                    );
                }
                let mut detail = written_line(lang, progress.map(|p| p.bytes));
                detail.push(
                    text::STEP_QUALITY
                        .get(lang)
                        .replace("{crf}", &crf.to_string()),
                );
                (label, progress.and_then(|p| p.fraction), detail)
            }
            Phase::B {
                step: phase_b::Step::Recalibrating { from, to },
                ..
            } => (
                text::STEP_RECALIBRATING
                    .get(lang)
                    .replace("{from}", &from.to_string())
                    .replace("{to}", &to.to_string()),
                None,
                Vec::new(),
            ),
        };
        let name = file_name(&working.job.source);

        centred(ui, "working", |ui| {
            theme::card(ui, |ui| {
                ui.set_width(ui.available_width());
                theme::muted(ui, &name);
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

                // THE SIZE, as it is written, and the quality producing it. The
                // bar says how far the encoder has got; these say whether the
                // result is going to be any use, and why this attempt differs
                // from the last one.
                if !detail.is_empty() {
                    ui.add_space(space::ROW);
                    theme::spec(ui, &detail.join(SEP));
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
        let blocked = bench.blocked();
        // Asked for and not yet carried out. Not a fault -- the file is
        // uploadable -- but not nothing either, and saying nothing about it put
        // "ready to upload" in green directly above "processing will crop it to
        // 1080x606".
        let requested = pending.iter().any(|item| !item.is_a_problem());
        let (_, bytes, facts) = bench.have();

        centred(ui, "bench", |ui| {
            theme::card(ui, |ui| {
                ui.set_width(ui.available_width());

                // FOUR LINES, AND EVERY ONE OF THEM HAS TO EARN ITS PLACE.
                //
                //   which file          a name, not a path -- the path is on the
                //                       screen where the file is chosen
                //   the headline        what, not merely that
                //   one line under it   why, or what just happened
                //   the specification   the facts, stated once each
                //
                // What this replaces was a labelled ledger: "just done / nothing
                // needed doing" directly above "compressed, 140.1 MB → 15.1 MB",
                // a headline that said "one more pass needed" without saying of
                // what, and the same number printed in two roundings three lines
                // apart. Labels cannot rescue a headline that says nothing.
                theme::muted(ui, &file_name(&bench.job.source));
                ui.add_space(space::BLOCK);

                let (hero, colour) = headline(bench, blocked, requested, lang);
                theme::heading(ui, hero, colour);
                ui.add_space(space::TIGHT);

                if let Some(under) = under_headline(bench, &pending, blocked, lang) {
                    theme::line(ui, &under, theme::INK);
                }

                // The number, only where an ENCODE produced it. The lossless
                // pass changes the weight by whatever the audio track happened
                // to be, and dramatising that in eighteen point crowds out the
                // line that matters.
                if let Some(done) = &bench.encoded {
                    ui.add_space(space::ROW);
                    theme::tally(ui, &megabytes(bench.job.bytes), &megabytes(done.bytes));
                    // Both directions. Hiding the growth would leave the
                    // charter to find it in the two numbers directly above.
                    if let Some(delta) = delta_line(bench, lang) {
                        ui.add_space(space::TIGHT);
                        theme::muted(ui, &delta);
                    }
                }

                // What the next press will do -- one line, not a bulleted list
                // that restates each problem underneath itself. Suppressed
                // wherever there is no next press: describing an action the
                // action bar is not offering is a description of nothing. The
                // two used to disagree about `Failed`, which drew a loud button
                // under a red headline with its explanation deliberately hidden.
                if !pending.is_empty() && bench.worth_running().is_some() {
                    ui.add_space(space::ROW);
                    theme::muted(ui, &will_line(&pending, lang));
                }

                ui.add_space(space::BLOCK);
                theme::spec(ui, &spec_line(bench, facts, bytes));

                if let Some(to) = &bench.saved {
                    ui.add_space(space::BLOCK);
                    theme::label(ui, text::SAVED_TO.get(lang));
                    ui.add_space(space::TIGHT);
                    theme::path(ui, &to.to_string_lossy());
                }
            });
        });
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

fn workbench_column(ui: &mut egui::Ui, lang: Lang, loaded: &mut Inspect) {
    let column = ui.max_rect();
    let controls = slice_bottom(column, TRANSPORT_H);
    let glass = egui::Rect::from_min_max(
        column.min,
        egui::pos2(column.right(), controls.top() - space::ROW),
    );

    // Real dimensions, always. This used to fall back to `(0, 0)` for a file
    // whose header could not be read -- a nonsense pair fed straight into the
    // crop geometry, for a screen that should never have been opened.
    let source = loaded.job.facts.picture();
    let texture = loaded.view.as_ref().and_then(|v| v.texture.clone());
    let mut crop = loaded.job.crop;
    region(ui, "the viewport", glass, |ui| {
        viewport(ui, texture.as_ref(), glass.size(), source, &mut crop);
    });
    loaded.job.crop = crop;
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
fn viewport(
    ui: &mut egui::Ui,
    texture: Option<&egui::TextureHandle>,
    room: egui::Vec2,
    source: (u32, u32),
    crop: &mut Option<crop::Choice>,
) {
    // Sensing a drag ONLY while a crop is chosen. With none set there is nothing
    // to move, and a picture that reacts to being dragged while doing nothing is
    // worse than one that does not react at all.
    let sense = if crop.is_some() {
        egui::Sense::click_and_drag()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(room, sense);
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
    let shown = egui::Rect::from_center_size(rect.center(), drawn);
    egui::Image::new(texture)
        .corner_radius(egui::CornerRadius::same(theme::RADIUS - 1))
        .paint_at(ui, shown);

    // ── the crop, shown on the picture it is a crop of ────────────────
    // Borrowed, not copied out. Taking a copy meant reaching back through the
    // same `Option` to write the drag into it, which needed an `expect` for a
    // case the line above had already ruled out.
    let Some(choice) = crop.as_mut() else {
        return;
    };
    let Some(window) = choice.window(source) else {
        return;
    };

    // Dragging moves the window along its ONE free axis. The other dimension is
    // already exactly the shape that was asked for, so allowing it to move would
    // only let the charter undo the thing they chose.
    // Measured in SOURCE pixels mapped onto the shown rectangle, not in the
    // preview texture's own pixels: the window is expressed in the source's
    // coordinates and the preview is a rounded-to-even approximation of them.
    let (sw, sh) = (source.0.max(1) as f32, source.1.max(1) as f32);
    if response.dragged() {
        let by = response.drag_delta();
        // THE FRAME FOLLOWS THE POINTER. Drag down, the frame goes down.
        //
        // The other convention -- drag the PICTURE, so it slides under a fixed
        // frame and the kept region appears to travel the opposite way -- is
        // what a phone's photo cropper does, and it is what this did. It is also
        // wrong HERE, because the picture is not what moves on screen: it is
        // pinned, and a bright rectangle sits on top of it. Anything drawn as an
        // object over a still image is grabbed as that object.
        let (moved, room) = match choice.along(source) {
            crop::Along::Horizontal => (by.x, shown.width() * (sw - window.width as f32) / sw),
            crop::Along::Vertical => (by.y, shown.height() * (sh - window.height as f32) / sh),
        };
        if room > 0.5 {
            choice.offset = (choice.offset + moved / room).clamp(0.0, 1.0);
        }
    }
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor_icon(match choice.along(source) {
            crop::Along::Horizontal => egui::CursorIcon::ResizeHorizontal,
            crop::Along::Vertical => egui::CursorIcon::ResizeVertical,
        });
    }

    // What SURVIVES is drawn plainly and what goes is dimmed, rather than the
    // other way round: the question the charter is answering is "what is left",
    // and an outline alone leaves them measuring by eye which side of it wins.
    let kept = egui::Rect::from_min_size(
        shown.min
            + egui::vec2(
                shown.width() * window.x as f32 / sw,
                shown.height() * window.y as f32 / sh,
            ),
        egui::vec2(
            shown.width() * window.width as f32 / sw,
            shown.height() * window.height as f32 / sh,
        ),
    );
    let veil = egui::Color32::from_black_alpha(150);
    let painter = ui.painter();
    for gone in [
        egui::Rect::from_min_max(shown.min, egui::pos2(shown.right(), kept.top())),
        egui::Rect::from_min_max(egui::pos2(shown.left(), kept.bottom()), shown.max),
        egui::Rect::from_min_max(shown.min, egui::pos2(kept.left(), shown.bottom())),
        egui::Rect::from_min_max(egui::pos2(kept.right(), shown.top()), shown.max),
    ] {
        if gone.is_positive() {
            painter.rect_filled(gone, 0, veil);
        }
    }
    painter.rect_stroke(
        kept,
        0,
        egui::Stroke::new(1.5, theme::ACCENT),
        egui::StrokeKind::Inside,
    );
}

/// Transport, timeline and what the trim currently means. Written bottom-up so
/// it can be dropped into a bottom-up layout unchanged.
fn transport(ui: &mut egui::Ui, lang: Lang, loaded: &mut Inspect) {
    // Destructured so the viewfinder, the position and the trim are three
    // separate borrows: the transport reads one and writes the others in the
    // same breath, which through `loaded.` alone the compiler cannot allow.
    let Inspect { job, at, view } = loaded;
    let preview = view.as_ref().map(|v| &v.preview);
    let (facts, trim) = (&job.facts, &mut job.trim);
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
fn verdict_card(
    ui: &mut egui::Ui,
    lang: Lang,
    loaded: &Inspect,
    missing: Option<&Absent>,
) -> Option<Option<crop::Choice>> {
    let opening = loaded.job.trim.cut_opening;
    let chose = loaded.job.crop;
    // Handed back rather than written here, so the card keeps its shared
    // reference and the caller owns every change to the stage.
    let mut picked = None;
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
        theme::path(ui, &loaded.job.source.to_string_lossy());
        ui.add_space(space::TIGHT);

        let facts = &loaded.job.facts;

        // SIZE IS NOT MENTIONED HERE, and its absence is the whole point of the
        // phase split. Whether this video fits in 80 MB is not a deterministic
        // question -- it depends on how compressible the footage turns out to
        // be, which nothing short of encoding it will say. Naming it on a screen
        // where every other line is settled would promise an answer Press does
        // not have yet.
        // WITH THE CROP, because the crop is chosen on this very card and the
        // scale is decided from the picture that survives it. Passing `None` made
        // the resolution bullet promise a scale computed for a frame that would
        // never exist -- and on a 2560x1960 source, picking 16:9 brings it to
        // exactly the pixel budget, so the bullet went on promising a scale that
        // the run does not perform at all.
        let window = chose.and_then(|c: crop::Choice| c.window(facts.picture()));
        let deterministic = findings(facts, window, lang);
        if deterministic.is_empty() {
            theme::heading(ui, text::PHASE_A_CLEAR.get(lang), theme::GOOD);
            ui.add_space(space::TIGHT);
            theme::muted(ui, text::PHASE_A_CLEAR_HINT.get(lang));
        } else {
            theme::heading(ui, text::VERDICT_WILL_FIX.get(lang), theme::NOTICE);
        }
        ui.add_space(space::TIGHT);
        theme::muted(ui, &shape(facts, loaded.job.bytes));

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

        // ── the shape, which is an OFFER ──────────────────────────────
        //
        // Never a finding, and never in the list above it: the gate checks a
        // pixel COUNT and has no opinion about shape, so a 4:3 background video
        // is accepted and will play. It simply will not fill the screen, and
        // whether losing a quarter of the picture is a fair trade for that is
        // not a call Press can make for somebody.
        //
        // Shown only when the source is neither of the two shapes players are
        // usually on -- offering a 16:9 crop of a 16:9 video is a decision put
        // in front of somebody for no reason.
        let source = facts.picture();
        if crop::is_unusual(source) {
            ui.add_space(space::BLOCK);
            theme::line(ui, text::CROP_OFFER.get(lang), theme::INK);
            ui.add_space(space::TIGHT);
            for shape in crop::Shape::ALL {
                let Some(window) = crop::window(source, shape, 0.5) else {
                    continue;
                };
                let (rw, rh) = shape.ratio();
                // The COST, on the option itself. It is the whole decision: from
                // a 4:3 source, 16:9 gives up a quarter of the picture and 16:10
                // gives up noticeably less, and nobody can weigh that without
                // the number.
                let label = text::CROP_OPTION
                    .get(lang)
                    .replace("{ratio}", &format!("{rw}:{rh}"))
                    .replace("{percent}", &format!("{:.0}", window.lost(source) * 100.0));
                let chosen = chose.is_some_and(|c: crop::Choice| c.shape == shape);
                if theme::pick(ui, &label, chosen).clicked() {
                    picked = Some(if chosen {
                        None
                    } else {
                        Some(crop::Choice::centred(shape))
                    });
                }
                ui.add_space(2.0);
            }
            if chose.is_some() {
                ui.add_space(space::TIGHT);
                theme::muted(ui, text::CROP_DRAG.get(lang));
            }
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
        if let Some(absent) = missing {
            ui.add_space(space::BLOCK);
            // WHICH cause, because they ask different things of the reader. One
            // is "this is a development build"; the other is "something on your
            // machine is holding the file", and that one is fixable by the
            // person reading it -- if they are told.
            let said = match absent {
                Absent::NotCarried => text::NO_ENCODER.get(lang).to_owned(),
                Absent::CouldNotUnpack(why) => {
                    text::ENCODER_STUCK.get(lang).replace("{reason}", why)
                }
            };
            ui.label(
                egui::RichText::new(said)
                    .size(theme::size::SMALL)
                    .color(theme::ALARM),
            );
        }
    });
    picked
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
                for (finding, fix) in findings(facts, None, lang) {
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
            quarter_turned: false,
        }
    }

    fn a_job() -> Job {
        Job {
            source: PathBuf::from(LONG),
            bytes: 98_000_000,
            facts: facts_of("h264", 3840, 2160, 120.0),
            trim: Trim {
                cut_opening: 1.5,
                keep_until: None,
            },
            crop: None,
        }
    }

    fn a_bench(applied: Vec<Applied>, encoded: Option<Box<Encoded>>, said: Option<Said>) -> Bench {
        Bench {
            job: a_job(),
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

    /// The layout is the same shape at every display scale.
    ///
    /// Press declares per-monitor v2 DPI awareness, which is what keeps a fixed
    /// 960x640 canvas of video sharp instead of letting Windows bitmap-stretch
    /// it. The cost of asking is that Press is handed the real scale factor and
    /// has to be correct at it -- and `region`'s fit assertion is a
    /// `debug_assert`, compiled out of the shipped build, so an overflow at 150%
    /// would clip in silence on the one screen nobody here is looking at.
    ///
    /// WHAT THIS ESTABLISHES, precisely: that the layout is scale-INVARIANT.
    /// egui lays out in points, so the canvas is 960x640 whatever the device
    /// does, and the measurement below is identical at 1.0 and at 2.0. It passes
    /// at absurd scales too, which is the correct result rather than a weak one.
    ///
    /// So it will not catch a bad layout -- `every_stage_fits_the_window` does
    /// that. It catches the day something starts reading `pixels_per_point` and
    /// the invariant this manifest setting rests on quietly stops holding.
    ///
    /// The scales are the ones Windows actually offers.
    #[test]
    fn the_layout_survives_every_display_scale() {
        for scale in [1.0_f32, 1.25, 1.5, 1.75, 2.0] {
            for lang in [Lang::En, Lang::ZhHans, Lang::De] {
                let ctx = egui::Context::default();
                ctx.set_pixels_per_point(scale);
                let mut press = Press::new(&ctx, lang, Err(Absent::NotCarried));

                // The tallest card there is: every problem at once, on the
                // longest-worded language, which is what decides whether the
                // readout column still holds its contents.
                press.stage = Stage::Bench(Box::new(Bench {
                    job: Job {
                        facts: facts_of("vp09", 3840, 2160, 120.0),
                        ..a_job()
                    },
                    material: Box::new(Material::for_test(
                        PathBuf::from(LONG),
                        396_000_000,
                        facts_of("vp09", 3840, 2160, 120.0),
                        vec![Applied::AudioDropped],
                    )),
                    ..a_bench(vec![], None, None)
                }));
                let painted = run(&mut press, &ctx, 3);

                // Proof this is not theatre: if egui ignored the request, every
                // scale below would be laying out at 1.0 and passing for it.
                assert_eq!(
                    ctx.pixels_per_point(),
                    scale,
                    "the context did not take the scale"
                );

                let screen =
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::Vec2::from(WINDOW));
                assert!(painted.is_positive(), "nothing painted at {scale}x");
                assert!(
                    screen.contains_rect(painted.shrink(0.5)),
                    "at {scale}x in {}, painted {painted:?} outside {screen:?}",
                    lang.endonym()
                );
            }
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
            let mut press = Press::new(&ctx, lang, Err(Absent::NotCarried));

            // Empty.
            run(&mut press, &ctx, 2);

            // Both phases running.
            let in_phase_b = |step| Phase::B {
                material: Box::new(Material::for_test(
                    PathBuf::from(LONG),
                    38_200_000,
                    facts_of("h264", 1080, 816, 24.0),
                    vec![Applied::AudioDropped],
                )),
                best: None,
                step,
            };
            let phases = [
                Phase::A(phase_a::Step::Copying(Some(sample))),
                Phase::A(phase_a::Step::Verifying),
                in_phase_b(phase_b::Step::Encoding {
                    crf: 26,
                    attempt: 2,
                    progress: Some(sample),
                }),
                in_phase_b(phase_b::Step::Recalibrating { from: 32, to: 20 }),
            ];
            for phase in phases {
                let (_tx, inbox) = channel();
                press.stage = Stage::Working(Box::new(Working {
                    job: a_job(),
                    phase,
                    cancel: Cancel::new(),
                    inbox,
                    work: Arc::new(Workspace::open().expect("a workspace")),
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
                // EVERY kind of change at once. Seven items in "what was done",
                // which is the longest that line can be, and it wraps -- so it
                // is the case that decides whether the card still fits.
                Bench {
                    job: Job {
                        crop: Some(crop::Choice::centred(crop::Shape::Wide)),
                        trim: Trim {
                            cut_opening: 1.2,
                            keep_until: None,
                        },
                        facts: facts_of("vp09", 5120, 2160, 120.0),
                        ..a_job()
                    },
                    material: Box::new(Material::for_test(
                        PathBuf::from(LONG),
                        390_000_000,
                        facts_of("vp09", 5120, 2160, 120.0),
                        vec![
                            Applied::AudioDropped,
                            Applied::TailCut {
                                was: 110.6,
                                now: 92.0,
                            },
                        ],
                    )),
                    ..a_bench(
                        vec![],
                        Some(Box::new(Encoded {
                            path: PathBuf::from(LONG),
                            bytes: 74_000_000,
                            facts: facts_of("av1", 2560, 1440, 60.0),
                            crf: 34,
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
                    job: Job {
                        facts: facts_of("vp09", 3840, 2160, 120.0),
                        ..a_job()
                    },
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
                // THE ENCODE THAT GREW. The screenshot that started all of this:
                // cropped, audio dropped, and the file came back heavier. The
                // headline must not say "compressed" and the delta must state
                // the growth -- neither of which any other bench here exercises,
                // because every other encode is smaller than its source.
                Bench {
                    job: Job {
                        bytes: 2_689_216,
                        crop: Some(crop::Choice::centred(crop::Shape::Wide)),
                        ..a_job()
                    },
                    ..a_bench(
                        vec![Applied::AudioDropped],
                        Some(Box::new(Encoded {
                            path: PathBuf::from(LONG),
                            bytes: 3_200_000,
                            facts: facts_of("av1", 720, 404, 30.0),
                            crf: 20,
                            encodes: 1,
                        })),
                        None,
                    )
                },
                a_bench(
                    vec![Applied::AudioDropped],
                    None,
                    Some(Said::MaterialWins {
                        av1_was: Some(112_000_000),
                    }),
                ),
                // The ordinary way AV1 loses: killed on its way past the
                // material's weight, so there is no measurement to quote.
                a_bench(
                    vec![Applied::AudioDropped],
                    None,
                    Some(Said::MaterialWins { av1_was: None }),
                ),
                // A crop that could not be carried out without gaining weight.
                Bench {
                    job: Job {
                        crop: Some(crop::Choice::centred(crop::Shape::Wide)),
                        ..a_job()
                    },
                    ..a_bench(
                        vec![Applied::AudioDropped],
                        None,
                        Some(Said::RequestDropped),
                    )
                },
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

    /// The whole card, as one list of rendered lines.
    fn card_lines(bench: &Bench, lang: Lang) -> Vec<String> {
        let pending = bench.pending();
        let blocked = bench.blocked();
        // Asked for and not yet carried out. Not a fault -- the file is
        // uploadable -- but not nothing either, and saying nothing about it put
        // "ready to upload" in green directly above "processing will crop it to
        // 1080x606".
        let requested = pending.iter().any(|item| !item.is_a_problem());
        let (_, bytes, facts) = bench.have();
        let mut lines = vec![file_name(&bench.job.source)];

        // THE CARD'S OWN FUNCTIONS, not a restatement of them. Every line below
        // comes from the code that paints it, so a wording bug shows up here
        // rather than in a screenshot.
        lines.push(headline(bench, blocked, requested, lang).0.to_owned());
        if let Some(under) = under_headline(bench, &pending, blocked, lang) {
            lines.push(under);
        }
        if let Some(done) = &bench.encoded {
            lines.push(format!(
                "{} → {}",
                megabytes(bench.job.bytes),
                megabytes(done.bytes)
            ));
            if let Some(delta) = delta_line(bench, lang) {
                lines.push(delta);
            }
        }
        if !pending.is_empty() && bench.worth_running().is_some() {
            lines.push(will_line(&pending, lang));
        }
        lines.push(spec_line(bench, facts, bytes));
        lines
    }

    /// Every weight stated on a card, as it is written.
    fn weights(lines: &[String]) -> Vec<String> {
        let mut found = Vec::new();
        for line in lines {
            let glyphs: Vec<char> = line.chars().collect();
            for (i, _) in line.match_indices(" MB") {
                let upto: Vec<char> = line[..i].chars().collect();
                let start = upto
                    .iter()
                    .rposition(|c| !(c.is_ascii_digit() || *c == '.'))
                    .map_or(0, |p| p + 1);
                let number: String = upto[start..].iter().collect();
                if !number.is_empty() {
                    found.push(number);
                }
            }
            let _ = glyphs;
        }
        found
    }

    /// The shapes a charter actually meets, in the two languages this project
    /// is read in most.
    fn every_shape() -> Vec<(&'static str, Bench)> {
        let big = || facts_of("h264", 5120, 2160, 60.0);
        let small = || facts_of("h264", 1080, 816, 24.0);
        let done = || facts_of("av1", 2956, 1246, 60.0);

        let shape = |source: Facts,
                     source_bytes: u64,
                     applied: Vec<Applied>,
                     material_bytes: u64,
                     encoded: Option<Box<Encoded>>| {
            let facts = source.clone();
            Bench {
                job: Job {
                    bytes: source_bytes,
                    facts: source,
                    trim: Trim::default(),
                    ..a_job()
                },
                material: Box::new(Material::for_test(
                    PathBuf::from(LONG),
                    material_bytes,
                    facts,
                    applied,
                )),
                encoded,
                said: None,
                saved: None,
                work: Arc::new(Workspace::open().expect("a workspace")),
            }
        };

        vec![
            (
                "audio removed, then compressed",
                shape(
                    small(),
                    98_500_000,
                    vec![Applied::AudioDropped],
                    94_000_000,
                    Some(Box::new(Encoded {
                        path: PathBuf::from(LONG),
                        bytes: 38_800_000,
                        facts: facts_of("av1", 1080, 816, 24.0),
                        crf: 20,
                        encodes: 2,
                    })),
                ),
            ),
            (
                "arrived correct",
                shape(
                    facts_of("av1", 1080, 816, 24.0),
                    38_800_000,
                    vec![],
                    38_800_000,
                    None,
                ),
            ),
            (
                "audio removed, still too big",
                shape(
                    small(),
                    98_500_000,
                    vec![Applied::AudioDropped],
                    94_000_000,
                    None,
                ),
            ),
            (
                "two problems, nothing done yet",
                shape(big(), 146_900_000, vec![], 146_900_000, None),
            ),
            // EVERY kind of change at once, which is the longest the "what was
            // done" line can be and the case that proves nothing is silently
            // left out of it.
            ("everything at once, and done", {
                let mut bench = shape(
                    facts_of("vp09", 5120, 2160, 120.0),
                    396_000_000,
                    vec![
                        Applied::AudioDropped,
                        Applied::TailCut {
                            was: 110.6,
                            now: 92.0,
                        },
                    ],
                    390_000_000,
                    Some(Box::new(Encoded {
                        path: PathBuf::from(LONG),
                        bytes: 74_000_000,
                        facts: facts_of("av1", 2560, 1440, 60.0),
                        crf: 34,
                        encodes: 2,
                    })),
                );
                bench.job.crop = Some(crop::Choice::centred(crop::Shape::Wide));
                bench.job.trim.cut_opening = 1.2;
                bench
            }),
            ("a crop asked for and not yet applied", {
                let mut bench = shape(
                    facts_of("h264", 1080, 816, 24.0),
                    23_700_000,
                    vec![Applied::AudioDropped],
                    23_700_000,
                    None,
                );
                bench.job.crop = Some(crop::Choice::centred(crop::Shape::Wide));
                bench
            }),
            (
                "two problems, then compressed",
                shape(
                    big(),
                    146_900_000,
                    vec![],
                    146_900_000,
                    Some(Box::new(Encoded {
                        path: PathBuf::from(LONG),
                        bytes: 15_800_000,
                        facts: done(),
                        crf: 20,
                        encodes: 2,
                    })),
                ),
            ),
            // THE SCREENSHOT THAT STARTED IT. Audio dropped, picture cropped,
            // and the file came back heavier. `Aim::Apply` no longer lets this
            // happen for a crop -- it drops the request instead, which is the
            // case below -- but `Aim::Repair` deliberately can: with nothing to
            // fall back on, a fixed file that gained a few percent beats a
            // broken one. So the two branches written for it still have to be
            // right, and every other encode here is smaller than its source.
            ("an encode that came back heavier", {
                let mut bench = shape(
                    facts_of("h264", 720, 720, 30.0),
                    2_689_216,
                    vec![Applied::AudioDropped],
                    2_399_618,
                    Some(Box::new(Encoded {
                        path: PathBuf::from(LONG),
                        bytes: 3_200_000,
                        facts: facts_of("av1", 720, 404, 30.0),
                        crf: 20,
                        encodes: 1,
                    })),
                );
                bench.job.crop = Some(crop::Choice::centred(crop::Shape::Wide));
                bench
            }),
            // And the answer to it: the request goes rather than the weight.
            ("a crop that could not be afforded", {
                let mut bench = shape(
                    facts_of("h264", 720, 720, 30.0),
                    2_689_216,
                    vec![Applied::AudioDropped],
                    2_399_618,
                    None,
                );
                bench.job.crop = Some(crop::Choice::centred(crop::Shape::Wide));
                bench.said = Some(Said::RequestDropped);
                bench
            }),
            // AV1 tried and lost, with no measurement to quote -- which is the
            // ordinary way it loses, since the encode is killed on its way past
            // the material's weight rather than allowed to finish.
            ("av1 lost, unmeasured", {
                let mut bench = shape(
                    small(),
                    38_800_000,
                    vec![Applied::AudioDropped],
                    38_800_000,
                    None,
                );
                bench.said = Some(Said::MaterialWins { av1_was: None });
                bench
            }),
            ("stopped by hand", {
                let mut bench = shape(
                    small(),
                    38_800_000,
                    vec![Applied::AudioDropped],
                    38_800_000,
                    None,
                );
                bench.said = Some(Said::Cancelled);
                bench
            }),
        ]
    }

    /// No weight is printed twice on one card.
    ///
    /// This defect has come back three times in three different disguises --
    /// `37.0 MB` in the tally and again in the specification; `375.4 MB` beside
    /// `376 MB`, the same number in two roundings; `140.1 MB` and `141 MB`. Each
    /// time it read as carelessness, because it is.
    ///
    /// Run with `--nocapture` to READ the copy rather than reason about it,
    /// which is how the last round of wording bugs were found.
    #[test]
    fn no_weight_is_stated_twice_on_one_card() {
        for lang in Lang::ALL {
            for (name, bench) in every_shape() {
                let lines = card_lines(&bench, lang);
                println!("\n### {name} [{}]", lang.endonym());
                for line in &lines {
                    println!("  {line}");
                    assert!(!line.trim().is_empty(), "{name}: an empty line");
                    assert!(
                        !line.contains('{') && !line.contains('}'),
                        "{name}: unfilled placeholder in {line:?}"
                    );
                }

                let mut seen: Vec<String> = Vec::new();
                for weight in weights(&lines) {
                    assert!(
                        !seen.contains(&weight),
                        "{name} [{}]: {weight} MB is stated twice:\n{}",
                        lang.endonym(),
                        lines.join("\n")
                    );
                    seen.push(weight);
                }
            }
        }
    }
}
