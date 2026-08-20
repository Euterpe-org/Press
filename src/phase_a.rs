//! Phase A: everything that is decided rather than negotiated.
//!
//! The five deterministic questions -- resolution, frame rate, codec, audio,
//! length -- have answers that follow from the file and the limits, with no
//! judgement and no search. Ask them once, get the same answer every time, and
//! the answer never changes again for this file. That is what makes this a phase
//! rather than a step: [`crate::phase_b`] may run many times, and everything
//! decided here holds across all of them.
//!
//! # What Phase A EXECUTES is narrower than what it DECIDES
//!
//! Press ships one encoder. `--enable-encoder=libsvtav1,rawvideo` -- there is no
//! libx264 and no libx265 in the binary, so the only two things Press can
//! produce are AV1 and an exact copy of what it was given.
//!
//! So rewriting the picture, for any reason, means AV1. If Phase A rescaled a
//! 4K source here it would have to encode it, and then Phase B would encode the
//! result again: two generations of loss for one video. (Measured early in this
//! project: a second AV1 generation came back at ~190% of the first for no
//! visible gain.)
//!
//! Phase A therefore executes only what a STREAM COPY can do -- drop the audio
//! track, cut the tail -- which is lossless and takes seconds. Scaling, frame
//! rate and the opening cut are decided here and applied by Phase B, inside the
//! same single encode that does the AV1 conversion. Their values are still
//! deterministic; only their execution is deferred, because there is exactly one
//! place they can happen without costing a generation.
//!
//! # The material
//!
//! Phase A yields a [`Material`]: either a file Press wrote, or the source
//! itself when there was nothing for a copy to do. Everything downstream reads
//! it without caring which, except for the one thing that matters -- Press
//! deletes what it made and never touches what it did not.

use std::path::{Path, PathBuf};

use crate::ffmpeg::{Cancel, Ffmpeg, Progress, Ran, Watching};
use crate::gate::Facts;
use crate::job::Job;
use crate::plan::{self, Encode, Target};
use crate::verify::{self, Expect};
use crate::workspace::Workspace;

/// Where a Phase A run has got to.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Reading the header. A few hundred bytes.
    Reading,
    /// The lossless copy.
    Copying(Option<Progress>),
    /// Reading the result back.
    Verifying,
}

/// What the copy actually did, for the diff the next screen shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Applied {
    /// The source's audio track was dropped. It was never played -- the game
    /// plays `music.ogg` -- so nothing is lost with it.
    AudioDropped,
    /// The tail past the music was cut.
    TailCut { was: f64, now: f64 },
}

/// The file Phase B works on.
///
/// Cloneable, and cheaply: it is a path and some numbers. That is what lets a
/// worker thread own one without any lifetime crossing the boundary.
#[derive(Debug, Clone)]
pub struct Material {
    path: PathBuf,
    /// Whether Press wrote this file. The source is never deleted.
    made: bool,
    pub bytes: u64,
    pub facts: Facts,
    /// Empty when the source needed nothing a copy can do.
    pub applied: Vec<Applied>,
}

impl Material {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// True when this is the charter's own file rather than something Press
    /// produced. The window says so, because "saved 0 bytes" on a file that was
    /// already correct is a confusing way to say "it was already correct".
    pub fn is_the_source(&self) -> bool {
        !self.made
    }

    /// One for a test to hold, without a real run to produce it.
    ///
    /// `made` is false, so nothing that touches this can delete the file it
    /// names -- which matters, because a test that got that wrong would delete
    /// whatever path it was pointed at.
    #[cfg(test)]
    pub fn for_test(path: PathBuf, bytes: u64, facts: Facts, applied: Vec<Applied>) -> Self {
        Self {
            path,
            made: false,
            bytes,
            facts,
            applied,
        }
    }
}

#[derive(Debug)]
pub enum Outcome {
    Settled(Box<Material>),
    Cancelled,
    /// The copy produced something that is not what it was asked for. Never a
    /// size complaint -- an oversized copy is expected and is Phase B's problem.
    Failed(String),
}

/// Run the deterministic pass.
///
/// Only the TAIL half of the charter's trim is applied here; moving the opening
/// needs `-ss`, which forces a re-encode, so it travels to Phase B with the rest
/// of the picture work.
pub fn run(
    ffmpeg: &Ffmpeg,
    job: &Job,
    work: &Workspace,
    cancel: &Cancel,
    mut on_step: impl FnMut(Step),
) -> Outcome {
    let Job {
        source,
        bytes: source_bytes,
        facts,
        trim,
        ..
    } = job;
    let (source, source_bytes) = (source.as_path(), *source_bytes);
    on_step(Step::Reading);

    // `keep_until` is an ABSOLUTE position in the source, which is what `-t`
    // with no `-ss` means. Not `trim.allowance()`, which is the KEPT LENGTH --
    // that was right when one command did the opening cut and the tail cut
    // together, and is wrong here: on a one-second clip with the opening handle
    // at 0.6s it made Phase A write `-t 0.4`, cutting the tail down to 0.4s, and
    // then Phase B took 0.6s off the front of a 0.4s file and produced nothing.
    //
    // The two halves of a trim are applied in two different places now, so each
    // has to take its own half and nothing else.
    let keep_until = trim.keep_until;

    // A copy with nothing to copy away is a copy of the whole file for no
    // reason -- seconds of disk on a 379 MB source, to produce a byte-identical
    // result. The source IS the material in that case.
    if !facts.has_audio && keep_until.is_none() {
        return Outcome::Settled(Box::new(Material {
            path: source.to_path_buf(),
            made: false,
            bytes: source_bytes,
            facts: facts.clone(),
            applied: Vec::new(),
        }));
    }

    let output = work.file("settled.mp4");
    // The pinned recipe, unchanged: `-c copy`, `-an`, `+faststart`, and `-t`
    // only when the tail is being cut. No `-ss` -- see `plan::command`.
    let copy_only = Target {
        // A copy cannot crop either.
        crop: None,
        // A copy cannot rescale or reframe, so the shape it is asked for is the
        // shape it already has. Carrying the real target here would only make
        // `command` build filters that `-c copy` ignores.
        size: None,
        fps: None,
        trim_to: keep_until,
        must_reencode: false,
        may_try_av1: false,
    };
    let cmd = plan::command(
        &source.to_string_lossy(),
        &output.to_string_lossy(),
        &copy_only,
        Encode::Copy,
    );

    // Not watched for size. This is the cheap candidate -- it rewrites no
    // picture and runs at disk speed -- and its exact weight is the number
    // Phase B is measured against, so stopping it early would save seconds and
    // cost the number that saves minutes.
    match ffmpeg.run(&cmd, Some(facts.duration), cancel, |p| {
        on_step(Step::Copying(Some(p)));
        Watching::Continue
    }) {
        Err(error) => {
            let _ = std::fs::remove_file(&output);
            return Outcome::Failed(format!("the lossless pass failed: {error}"));
        }
        // A partial MP4 left on disk is worse than none: it looks like a result.
        //
        // The two stopped-early cases are ONE case here. The watcher above
        // returns `Continue` unconditionally, so nothing can abandon this run --
        // and were that ever untrue, "it stopped without finishing" is precisely
        // what `Cancelled` already means. This arm used to be an `unreachable!`,
        // which is a panic on a worker thread: it would have taken the window
        // down with it, and left the charter looking at a progress bar that had
        // stopped moving for a reason nothing on screen could name.
        Ok(Ran::Cancelled | Ran::Abandoned) => {
            let _ = std::fs::remove_file(&output);
            return Outcome::Cancelled;
        }
        Ok(Ran::ToCompletion) => {}
    }

    on_step(Step::Verifying);
    let seconds = keep_until.unwrap_or(facts.duration);
    let checked = match verify::judge(
        &output,
        &Expect {
            // A copy changes neither the frame size nor the codec, so both are
            // held to exactly what went in. Anything else means `-c copy` did
            // not do what `-c copy` does.
            size: facts.picture(),
            codec: facts.codec,
            seconds,
            // Absolute for a trim, relative for a whole file: a second of slack
            // on a one-second fixture would accept a video of any length.
            tolerance: plan::TRIM_TOLERANCE.max(seconds * 0.02),
        },
    ) {
        Ok(checked) => checked,
        Err(broken) => {
            let _ = std::fs::remove_file(&output);
            return Outcome::Failed(broken.0);
        }
    };

    // A stream copy rewrites no picture, so its output can only grow by
    // container overhead. Past that, something happened that a remux has no
    // business doing.
    if checked.bytes as f64 > source_bytes as f64 * plan::LOSSLESS_GROWTH_MARGIN {
        let _ = std::fs::remove_file(&output);
        return Outcome::Failed(format!(
            "the lossless pass grew to {:.0}% of the source, which a remux cannot explain",
            checked.bytes as f64 / source_bytes as f64 * 100.0
        ));
    }

    let mut applied = Vec::new();
    if facts.has_audio {
        applied.push(Applied::AudioDropped);
    }
    if let Some(now) = keep_until {
        applied.push(Applied::TailCut {
            was: facts.duration,
            now,
        });
    }

    Outcome::Settled(Box::new(Material {
        path: output,
        made: true,
        bytes: checked.bytes,
        facts: checked.facts,
        applied,
    }))
}
