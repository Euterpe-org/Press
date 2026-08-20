//! What has to change about this file, and how much it is allowed to cost.
//!
//! A port of `Euterpe.Web/scripts/video_transcode.py`'s `plan()` and its ffmpeg
//! recipe. The reason to mirror it rather than invent one: a video standardised
//! by Press should be the SAME FILE the server's periodic round would have
//! produced, so that round has nothing left to do. Every chart it does not have
//! to touch is one that never carries a `video_holders_stale` debt.
//!
//! # The two populations, treated oppositely
//!
//! Four constraints are mandatory -- no audio track, inside the pixel limit,
//! inside the frame-rate limit, no longer than its music. **AV1 is not**
//! (K's ruling, 2026-08-18). That splits every file in two:
//!
//! * **Re-encoding is FORCED** by a resolution or frame rate outside the gate --
//!   neither can be changed by copying streams -- or by a codec the gate refuses
//!   outright (VP9, mp4v). AV1 is the only way out and the result is kept
//!   whatever it weighs, because the alternative is a file that stays illegal.
//! * **Re-encoding is OFFERED** when the file already conforms and only its
//!   audio or its tail is wrong. A stream copy fixes both exactly and for free,
//!   so AV1 is encoded anyway and kept ONLY if it comes back smaller.
//!
//! CRF is a quality target, not a size target, so a source already compressed
//! below it comes back BIGGER. Measured across eight real chart videos the
//! results ranged from 13% to 140% of the original, and source bitrate did not
//! predict which. That is why the size test belongs to the codec preference
//! alone and never to the constraints.
//!
//! An already-AV1 source is never re-encoded at all: a second generation of loss
//! measured ~190% and buys nothing.
//!
//! # This module is a MIRROR, and some of it Press never drives
//!
//! `plan()` is law -- `tests/plan.rs` asserts every scenario the server plans,
//! field for field, from a recorded snapshot -- so the surface stays whatever
//! the server's surface is, whether or not Press has a use for all of it.
//! Reading it as though every part were live is how a stale comment gets
//! believed, so the divergences are named here rather than discovered:
//!
//! * `allowance` and the `trim_to` it produces are the server's duration rule.
//!   Press passes `None` at every call site: the gate does not judge duration
//!   (K, 2026-08-17) and a charter's length decision comes from the scrubber
//!   instead, which [`crate::phase_a`] applies directly.
//! * [`CRF_ESCALATION`] is the server's ladder. Press does not walk it --
//!   [`crate::phase_b`] searches the measured CRF/size law instead, which the
//!   fixed ladder could not do and which is why it was replaced.
//! * `must_reencode` and `may_try_av1` describe the populations above.
//!   [`crate::phase_b::aim`] decides Press's own version of that question, from
//!   the same facts and with more cases than two.
//!
//! Everything else here is on the path every run takes.

use crate::crop;
use crate::gate::{Facts, limits};

/// The quality dial, and the only knob worth touching. Chosen by K against
/// footage that is dimmed, scrimmed and playing behind gameplay.
pub const CRF: u32 = 32;

/// Speed/efficiency balance. ~3.5x realtime on a desktop; measured 8.7x on a
/// 24-core machine for 720p and 1.9x for 1080p.
pub const PRESET: u32 = 6;

/// The server's ladder for a file that will not fit: try 38, then 44.
///
/// **Press does not walk it.** Three fixed guesses cannot use what the first one
/// measured, and could not go finer than 32 at all -- a 94 MB source came back
/// at 19.2 MB having spent a quarter of the allowance. [`crate::phase_b`]
/// inverts the measured CRF/size law instead.
///
/// Kept, and pinned by `tests/plan.rs`, because it is the server's number: if
/// the round's ladder moves, that is worth being told about even though nothing
/// here reads it.
pub const CRF_ESCALATION: [u32; 2] = [38, 44];

/// A stream copy rewrites no picture, so its output can only grow by container
/// overhead -- `+faststart` moving the moov to the front measured 17 KB on a
/// 6 MB file. Past this, something other than overhead happened.
pub const LOSSLESS_GROWTH_MARGIN: f64 = 1.02;

/// A trim lands on a packet boundary, so it can sit a fraction of a second past
/// the mark. A second of overshoot is not a failure.
pub const TRIM_TOLERANCE: f64 = 1.0;

/// The shape this particular video has to end up in.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    /// The charter's crop, applied BEFORE anything else. `None` keeps the whole
    /// frame.
    pub crop: Option<crop::Window>,
    /// `None` keeps the frame size the crop left behind.
    pub size: Option<(u32, u32)>,
    /// `None` keeps the source's frame rate.
    pub fps: Option<u32>,
    /// `None` keeps the source's length.
    pub trim_to: Option<f64>,
    /// Geometry, frame rate, or a codec the gate refuses: a stream copy cannot
    /// fix any of them.
    pub must_reencode: bool,
    /// Conforms already, so AV1 has to earn its place on size.
    pub may_try_av1: bool,
}

impl Target {
    pub fn is_noop(&self) -> bool {
        self.crop.is_none()
            && self.size.is_none()
            && self.fps.is_none()
            && self.trim_to.is_none()
            && !self.must_reencode
    }

    /// The frame size the result will actually have, given what went in.
    ///
    /// Crop then scale, in that order, which is the order the filters run in.
    /// Reading `size` alone is wrong whenever a crop happened without one --
    /// the picture is smaller and nothing said so.
    pub fn frame_size(&self, source: (u32, u32)) -> (u32, u32) {
        if let Some(size) = self.size {
            return size;
        }
        match self.crop {
            Some(window) => (window.width, window.height),
            None => source,
        }
    }
}

/// What has to change, and whether re-encoding is forced or merely offered.
///
/// `allowance` is how long this video may run -- the song's own length plus
/// `limits::OVERRUN_ALLOWANCE_SECONDS`. `None` means no opinion, and the video
/// keeps its own length: **the gate does not judge duration and Press must never
/// imply that it does.**
///
/// `crop` is the charter's own decision and is applied FIRST, so everything
/// after it is decided on the picture that will actually exist. A crop can take
/// a source under the pixel budget on its own -- 2560x1960 is over it and the
/// 16:9 window out of it is 2560x1440, exactly at the line -- and it can equally
/// leave one still over, which then scales as usual. Deciding the scale from the
/// SOURCE's dimensions would get both cases wrong: a needless scale in the first
/// and, in the second, a scale factor computed for a picture that no longer
/// exists by the time it runs.
pub fn plan(facts: &Facts, allowance: Option<f64>, crop: Option<crop::Window>) -> Target {
    let (width, height) = match crop {
        Some(window) => (window.width, window.height),
        None => facts.picture(),
    };

    let pixels = u64::from(width) * u64::from(height);
    let size = (pixels > limits::MAX_PIXELS).then(|| {
        let scale = (limits::MAX_PIXELS as f64 / pixels as f64).sqrt();
        // Down to EVEN dimensions: yuv420p subsamples by two, and rounding DOWN
        // keeps the result inside the budget rather than a pixel over it. One
        // factor for both axes, so a crop's shape survives the scaling that
        // follows it.
        (even_floor(width, scale), even_floor(height, scale))
    });

    let fps = crate::gate::probe::over_fps_limit(facts.fps).then_some(limits::MAX_FPS);
    let trim_to = allowance.filter(|a| facts.duration > *a);

    // A crop rewrites the picture, so it forces the encode by itself even when
    // everything else about the file is already legal.
    let must_reencode =
        crop.is_some() || size.is_some() || fps.is_some() || !limits::codec_allowed(facts.codec);
    let may_try_av1 = !must_reencode && facts.codec != Some("av1");

    Target {
        crop,
        size,
        fps,
        trim_to,
        must_reencode,
        may_try_av1,
    }
}

/// Python's `max(2, int(x * scale) // 2 * 2)`: truncate toward zero, then round
/// down to even, then refuse to vanish.
fn even_floor(value: u32, scale: f64) -> u32 {
    let scaled = (f64::from(value) * scale) as u32;
    (scaled / 2 * 2).max(2)
}

/// Which encoder a candidate is asking for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encode {
    /// Stream copy: audio and tail fixed, picture untouched, exact and free.
    Copy,
    /// Re-encode to AV1 at this CRF.
    Av1(u32),
}

/// The ffmpeg argument list, byte for byte what the server's round runs.
///
/// **There is no `-ss` in this function and there must never be one.** The trim
/// cuts the TAIL and never the opening -- K's red line -- and `-t` with no `-ss`
/// is what makes that a property of the command rather than a convention someone
/// has to remember. `tests/plan.rs` asserts its absence.
pub fn command(input: &str, output: &str, target: &Target, encode: Encode) -> Vec<String> {
    let mut cmd: Vec<String> = ["ffmpeg", "-y", "-loglevel", "error", "-i", input]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    if let Some(trim_to) = target.trim_to {
        cmd.push("-t".to_owned());
        cmd.push(format!("{trim_to:.3}"));
    }

    match encode {
        Encode::Copy => {
            cmd.push("-c".to_owned());
            cmd.push("copy".to_owned());
        }
        Encode::Av1(crf) => {
            // CROP FIRST. Everything after it works on the picture the crop
            // leaves behind, which is the picture the scale factor was
            // computed for.
            let mut filters = Vec::new();
            if let Some(window) = target.crop {
                filters.push(format!(
                    "crop={}:{}:{}:{}",
                    window.width, window.height, window.x, window.y
                ));
            }
            if let Some((w, h)) = target.size {
                filters.push(format!("scale={w}:{h}:flags=lanczos"));
            }
            if let Some(fps) = target.fps {
                filters.push(format!("fps={fps}"));
            }
            if !filters.is_empty() {
                cmd.push("-vf".to_owned());
                cmd.push(filters.join(","));
            }
            for arg in ["-c:v", "libsvtav1", "-crf"] {
                cmd.push(arg.to_owned());
            }
            cmd.push(crf.to_string());
            cmd.push("-preset".to_owned());
            cmd.push(PRESET.to_string());
            cmd.push("-pix_fmt".to_owned());
            cmd.push("yuv420p".to_owned());
        }
    }

    // `-an`: the video's own audio is never played -- the game plays music.ogg
    // and the mod sets audioOutputMode = None -- so it is dropped, not
    // re-encoded. `+faststart` puts the moov box first; costs nothing.
    for arg in ["-an", "-movflags", "+faststart", output] {
        cmd.push(arg.to_owned());
    }
    cmd
}

/// The recipe plus an opening cut -- something the server's round NEVER does.
///
/// K's red line is that the standardisation round only ever takes the tail: it
/// touches charts belonging to other people, and shifting their opening against
/// the music is damage nobody asked for. [`command`] therefore contains no
/// `-ss` at all and a test asserts it never will.
///
/// This is the other case, and it is a different act. The charter is the author,
/// looking at their own footage, deciding that its first two seconds are a slate
/// that should not be there. Refusing them that is not caution, it is a tool
/// deciding it knows their video better than they do. So the capability exists,
/// in a function whose name says what it does, reachable only when someone has
/// dragged the opening handle themselves.
///
/// Two consequences the window has to state out loud before anyone gets here:
///
/// * The picture ends up **earlier against the music** by exactly `cut_opening`.
///   The mod keeps background video time-aligned with the song, so this is a
///   deliberate desync and only right when the opening was never meant to line
///   up.
/// * **It always re-encodes.** A stream copy can only start at a keyframe -- two
///   seconds apart in a typical H.264 export, five in AV1 -- so copying would
///   either land somewhere the charter did not choose or leave a broken opening.
///   Frame accuracy is worth the wait; guessing at seconds is not.
///
/// `-ss` sits AFTER `-i`, which decodes and discards up to the mark rather than
/// jumping to the nearest keyframe. Slower, and exact.
pub fn command_with_opening_cut(
    input: &str,
    output: &str,
    target: &Target,
    crf: u32,
    cut_opening: f64,
    keep_seconds: Option<f64>,
) -> Vec<String> {
    debug_assert!(
        cut_opening > 0.0,
        "an opening cut of zero belongs in `command`"
    );

    let mut cmd: Vec<String> = ["ffmpeg", "-y", "-loglevel", "error", "-i", input]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();

    cmd.push("-ss".to_owned());
    cmd.push(format!("{cut_opening:.3}"));
    if let Some(keep) = keep_seconds {
        cmd.push("-t".to_owned());
        cmd.push(format!("{keep:.3}"));
    }

    let mut filters = Vec::new();
    if let Some(window) = target.crop {
        filters.push(format!(
            "crop={}:{}:{}:{}",
            window.width, window.height, window.x, window.y
        ));
    }
    if let Some((w, h)) = target.size {
        filters.push(format!("scale={w}:{h}:flags=lanczos"));
    }
    if let Some(fps) = target.fps {
        filters.push(format!("fps={fps}"));
    }
    if !filters.is_empty() {
        cmd.push("-vf".to_owned());
        cmd.push(filters.join(","));
    }
    for arg in ["-c:v", "libsvtav1", "-crf"] {
        cmd.push(arg.to_owned());
    }
    cmd.push(crf.to_string());
    cmd.push("-preset".to_owned());
    cmd.push(PRESET.to_string());
    cmd.push("-pix_fmt".to_owned());
    cmd.push("yuv420p".to_owned());

    for arg in ["-an", "-movflags", "+faststart", output] {
        cmd.push(arg.to_owned());
    }
    cmd
}

/// What the charter asked for on the timeline.
///
/// Separate from [`Target`] on purpose. `Target` is the SERVER's decision,
/// pinned scenario by scenario against its own planner; this is the one thing
/// Press decides for itself, and keeping them apart is what stops an authored
/// choice from drifting into the part that has to match.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Trim {
    /// Seconds removed from the OPENING. Zero leaves it alone, and zero is the
    /// default -- this one is never chosen for the charter.
    pub cut_opening: f64,
    /// Where the kept region ends, measured from the start of the SOURCE.
    /// `None` keeps everything to the end.
    pub keep_until: Option<f64>,
}

impl Trim {
    pub fn is_untouched(&self) -> bool {
        self.cut_opening <= 0.0 && self.keep_until.is_none()
    }

    /// True when the opening moves, which forces a re-encode.
    pub fn moves_the_opening(&self) -> bool {
        self.cut_opening > 0.0
    }

    /// How long the result runs.
    pub fn kept_seconds(&self, source_duration: f64) -> f64 {
        (self.keep_until.unwrap_or(source_duration) - self.cut_opening).max(0.0)
    }

    /// The value [`plan`] wants: a maximum length for the OUTPUT, or `None`.
    ///
    /// With no opening cut this is simply where the tail stops. With one, the
    /// output is shorter than the source by the cut as well, and `plan` is being
    /// asked only whether a trim happens at all.
    pub fn allowance(&self, source_duration: f64) -> Option<f64> {
        if self.is_untouched() {
            return None;
        }
        Some(self.kept_seconds(source_duration))
    }
}

/// Progress reporting, spliced in by the runner rather than baked into
/// [`command`].
///
/// These flags are output-neutral -- they change what ffmpeg SAYS, never what it
/// writes -- and keeping them out of `command()` is what lets that function stay
/// comparable, argument for argument, with the recipe the server runs.
///
/// `-progress pipe:1` writes `key=value` lines to stdout (`out_time_us`,
/// `total_size`, `speed`, and `progress=continue|end`); `-nostats` silences the
/// human-readable line that would otherwise interleave with them on stderr.
pub const PROGRESS_FLAGS: [&str; 3] = ["-progress", "pipe:1", "-nostats"];

/// Noise on stderr that is not a diagnostic.
///
/// SVT-AV1 prints a banner and its whole configuration through its own logger,
/// which `-loglevel error` does not reach -- those lines are emitted by the
/// encoder library, not by ffmpeg. The server's script sends stderr to
/// /dev/null and never notices; Press keeps stderr in order to explain a real
/// failure, so it has to drop these itself or a genuine error arrives buried
/// under twenty lines of configuration.
pub fn is_encoder_noise(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with("Svt[info]:") || line.starts_with("Svt[warn]:")
}
