//! Making a file the server will accept, and proving it before saying so.
//!
//! Two candidates compete. A **stream copy** fixes an audio track or an
//! over-long tail exactly, for free, without touching a pixel -- and that is the
//! whole fix for the great majority of files (286 of 349 in the live library had
//! an audio track as their only violation). **AV1** is encoded as well, and kept
//! only if it comes back smaller than that exact copy.
//!
//! When the geometry, the frame rate or the codec is wrong there is no copy to
//! compete with: re-encoding is the only way out and its result is kept whatever
//! it weighs, because the alternative is a file that stays illegal.
//!
//! Nothing is emitted that the gate would refuse. Every candidate is re-read
//! through the same [`gate`] that judged the input, and a tool that can hand
//! back a file its own gate rejects is worse than one that does not check at
//! all.
//!
//! # There are no options here, deliberately
//!
//! Resolution, frame rate, codec and size are not the charter's problem. They
//! are the server's rules, they have exactly one correct resolution each, and a
//! checkbox asking a musician whether to downscale to 2560x1440 is a question
//! with a right answer being asked of someone who has no way to know it. All
//! four are fixed automatically and reported afterwards, never offered.
//!
//! The only thing the charter decides is the trim -- because only they know what
//! their video is supposed to show -- and whether to wait for the AV1
//! comparison. Anything else that grows a switch here is a mistake.
//!
//! # Cancelling means different things to the two populations
//!
//! For a file whose only problem is an audio track or a tail, the stream copy is
//! ALREADY a fully compliant result: cancelling costs nothing but the chance of
//! a smaller file. For a file that must be re-encoded, the encode IS the fix and
//! there is no compliant copy to fall back on -- cancelling yields
//! [`Outcome::Cancelled`] and nothing on disk. The window has to word the button
//! accordingly; it is not the same button.

use std::path::{Path, PathBuf};

use crate::ffmpeg::{Cancel, Ffmpeg, Progress, Ran};
use crate::gate::{self, Facts};
use crate::plan::{self, Encode, Target, Trim};

/// Where a run has got to, for the window to render.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// Reading the header. No decoding; a few hundred bytes.
    Reading,
    /// The exact, free candidate.
    Copying(Option<Progress>),
    /// The competing one. `attempt` counts from 1 and only advances when the
    /// result did not fit under the size limit.
    Encoding {
        crf: u32,
        attempt: usize,
        progress: Option<Progress>,
    },
    /// Re-reading the candidates through the gate.
    Verifying,
}

/// What was done, in terms a charter can be told.
#[derive(Debug, Clone, PartialEq)]
pub enum Treatment {
    /// The picture was not touched: the audio track and/or the tail were
    /// removed by a remux.
    StreamCopy,
    /// Re-encoded. `crf` is worth showing -- anything past the first rung was
    /// forced there by the size limit and has visibly paid for it.
    Av1 { crf: u32 },
    /// AV1 was tried and lost on size, so the source's own codec was kept and
    /// only the remux applied. `av1_ratio` is what it would have weighed.
    KeptOwnCodec { av1_ratio: f64 },
}

#[derive(Debug)]
pub struct Produced {
    pub path: PathBuf,
    pub bytes: u64,
    pub facts: Facts,
    pub treatment: Treatment,
    /// Fixes that were applied, for the summary line.
    pub audio_dropped: bool,
    pub scaled_to: Option<(u32, u32)>,
    pub frame_rate_clamped: Option<u32>,
    pub trimmed_to: Option<f64>,
    /// Seconds removed from the OPENING at the charter's request. Zero for
    /// everything Press does on its own -- the automatic path never touches it.
    pub opening_cut: f64,
    /// The AV1 comparison was stopped on request, so this result is the exact
    /// stream copy and might not be the smallest one available. Worth saying:
    /// the file is correct either way, but the charter chose not to find out.
    pub av1_cancelled: bool,
}

#[derive(Debug)]
pub enum Outcome {
    /// Nothing to do: it already conforms, in AV1, with no audio and no
    /// over-run. The only case where Press writes nothing at all.
    AlreadyStandard,
    Produced(Box<Produced>),
    /// Stopped on request with nothing to show for it.
    ///
    /// Only reachable when a re-encode was FORCED. Where a stream copy was
    /// possible it has already produced a compliant file, and cancelling the
    /// AV1 comparison ships that instead.
    Cancelled,
    /// Every candidate came out over the size limit, including the bottom rung
    /// of the quality ladder.
    ///
    /// Not a defect and not a bug: a hundred and ten seconds of 1080p, or eight
    /// minutes of anything, does not fit in 80 MB at a quality worth shipping.
    /// The remaining lever is the one Press will not pull on its own -- the
    /// length -- so the charter is told the number and asked to trim.
    StillTooLarge {
        /// The smallest thing that was actually produced, in bytes.
        smallest: u64,
    },
    /// Still non-standard, and a person has to look. Never a silent skip.
    Failed(String),
}

/// Why a candidate did not make it into the contest.
///
/// The distinction is the whole point: **being too large is not a failure**, it
/// is a candidate that lost. Conflating the two is what made a 94 MB source
/// stop dead at the stream copy and never reach the encoder that would have
/// fixed it -- the copy came back legal in every way except size, that was
/// reported as an error, and the run ended there.
enum Rejected {
    /// Legal in every respect except the size limit. Carries what it weighed.
    TooLarge(u64),
    /// Something actually went wrong: unreadable, wrong codec, wrong size, an
    /// audio track that survived. A person has to look at these.
    Broken(String),
}

/// One run, start to finish. Blocking; the caller owns the thread.
pub fn standardise(
    ffmpeg: &Ffmpeg,
    source: &Path,
    output: &Path,
    trim: Trim,
    cancel: &Cancel,
    mut on_step: impl FnMut(Step),
) -> Outcome {
    on_step(Step::Reading);
    let bytes = match std::fs::read(source) {
        Ok(bytes) => bytes,
        Err(error) => return Outcome::Failed(format!("could not read the file: {error}")),
    };
    let source_bytes = bytes.len() as u64;
    let Some(before) = gate::probe(&bytes) else {
        return Outcome::Failed("not a readable MP4 — re-export it as a standard MP4".to_owned());
    };
    drop(bytes);

    let mut target = plan::plan(&before, trim.allowance(before.duration));

    // An opening cut cannot be done by copying streams -- a copy can only start
    // at a keyframe, seconds away from wherever the charter put the handle -- so
    // asking for one removes the exact candidate from the contest entirely and
    // the encode becomes the only path. See `plan::command_with_opening_cut`.
    if trim.moves_the_opening() {
        target.must_reencode = true;
        target.may_try_av1 = false;
    }

    if before.codec == Some("av1") && target.is_noop() && !before.has_audio {
        return Outcome::AlreadyStandard;
    }

    let scratch = output.parent().unwrap_or(Path::new("."));
    let stem = output
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("video");
    let copy_path = scratch.join(format!(".{stem}.copy.mp4"));
    let av1_path = scratch.join(format!(".{stem}.av1.mp4"));
    let source_str = source.to_string_lossy().into_owned();

    let verify = Verify {
        before: &before,
        target: &target,
    };
    let mut candidates: Vec<Candidate> = Vec::new();
    // The smallest thing that was produced and still refused on size. Kept so
    // that "nothing survived" can say the number rather than shrugging.
    let mut smallest_over: Option<u64> = None;

    // ── the exact candidate ──────────────────────────────────────────────
    // Only possible when nothing about the geometry has to move.
    if !target.must_reencode {
        let cmd = plan::command(
            &source_str,
            &copy_path.to_string_lossy(),
            &target,
            Encode::Copy,
        );
        match ffmpeg.run(&cmd, Some(before.duration), cancel, |p| {
            on_step(Step::Copying(Some(p)));
        }) {
            Err(error) => {
                let _ = std::fs::remove_file(&copy_path);
                return Outcome::Failed(format!("the stream copy failed: {error}"));
            }
            // A partial MP4 left on disk is worse than none: it looks like a
            // result. The copy takes a fraction of a second, so this is rare.
            Ok(Ran::Cancelled) => {
                let _ = std::fs::remove_file(&copy_path);
                return Outcome::Cancelled;
            }
            Ok(Ran::ToCompletion) => {}
        }

        on_step(Step::Verifying);
        match verify.judge(&copy_path, Encode::Copy) {
            Err(Rejected::Broken(reason)) => {
                let _ = std::fs::remove_file(&copy_path);
                return Outcome::Failed(reason);
            }
            // A remux can only drop the audio track, so when the picture alone
            // is over the limit there is nothing a copy can do about it -- which
            // is precisely the case the AV1 competitor exists for. Treating this
            // as an error meant a 94 MB source stopped here and never reached
            // the encoder that would have fixed it.
            Err(Rejected::TooLarge(weighed)) => {
                smallest_over = Some(smallest_over.map_or(weighed, |s: u64| s.min(weighed)));
                let _ = std::fs::remove_file(&copy_path);
            }
            Ok(candidate) => {
                // A stream copy rewrites no picture, so it can only grow by
                // container overhead. Past that, something happened that a
                // remux has no business doing.
                if candidate.bytes as f64 > source_bytes as f64 * plan::LOSSLESS_GROWTH_MARGIN {
                    let ratio = candidate.bytes as f64 / source_bytes as f64;
                    let _ = std::fs::remove_file(&copy_path);
                    return Outcome::Failed(format!(
                        "the stream copy grew to {:.0}% of the source, which a remux cannot \
                         explain",
                        ratio * 100.0
                    ));
                }
                candidates.push(candidate);
            }
        }
    }

    // ── the AV1 candidate ────────────────────────────────────────────────
    let mut av1_bytes = None;
    let mut av1_cancelled = false;
    if target.must_reencode || target.may_try_av1 {
        let ladder: Vec<u32> = std::iter::once(plan::CRF)
            .chain(plan::CRF_ESCALATION)
            .collect();
        let mut last_error = None;

        for (attempt, crf) in ladder.iter().copied().enumerate() {
            // Also checked before each rung: a cancel that lands during
            // verification must not be answered by starting another encode.
            if cancel.requested() {
                av1_cancelled = true;
                break;
            }

            let destination = av1_path.to_string_lossy();
            let cmd = if trim.moves_the_opening() {
                plan::command_with_opening_cut(
                    &source_str,
                    &destination,
                    &target,
                    crf,
                    trim.cut_opening,
                    target.trim_to,
                )
            } else {
                plan::command(&source_str, &destination, &target, Encode::Av1(crf))
            };
            let ran = ffmpeg.run(
                &cmd,
                Some(target.trim_to.unwrap_or(before.duration)),
                cancel,
                |p| {
                    on_step(Step::Encoding {
                        crf,
                        attempt: attempt + 1,
                        progress: Some(p),
                    });
                },
            );
            match ran {
                Err(error) => {
                    last_error = Some(error);
                    break;
                }
                Ok(Ran::Cancelled) => {
                    av1_cancelled = true;
                    break;
                }
                Ok(Ran::ToCompletion) => {}
            }

            on_step(Step::Verifying);
            match verify.judge(&av1_path, Encode::Av1(crf)) {
                Ok(candidate) => {
                    av1_bytes = Some(candidate.bytes);
                    candidates.push(candidate);
                }
                // Only being over the SIZE limit is worth another pass at a
                // lower quality. Nothing else on the gate's list is fixed by
                // CRF, so walking the ladder for those would spend minutes to
                // fail twice more.
                Err(Rejected::TooLarge(weighed)) => {
                    smallest_over = Some(smallest_over.map_or(weighed, |s: u64| s.min(weighed)));
                    let _ = std::fs::remove_file(&av1_path);
                    if attempt + 1 < ladder.len() {
                        continue;
                    }
                }
                Err(Rejected::Broken(reason)) => {
                    let _ = std::fs::remove_file(&av1_path);
                    if target.must_reencode {
                        // There is no other way to make this file legal.
                        let _ = std::fs::remove_file(&copy_path);
                        return Outcome::Failed(reason);
                    }
                    // Merely offered, and a copy may already be in the contest.
                }
            }
            break;
        }

        if av1_cancelled {
            let _ = std::fs::remove_file(&av1_path);
        }
        if let Some(error) = last_error {
            let _ = std::fs::remove_file(&av1_path);
            if target.must_reencode {
                let _ = std::fs::remove_file(&copy_path);
                return Outcome::Failed(format!("the encode failed: {error}"));
            }
        }
    }

    // Cancelling forfeits a smaller file where a compliant copy already exists,
    // and forfeits EVERYTHING where one does not -- a forced re-encode, or a
    // source so heavy that the copy was over the limit too. The test is simply
    // whether anything is left to ship, which is what the charter was told the
    // button would decide.
    if av1_cancelled && candidates.is_empty() {
        let _ = std::fs::remove_file(&copy_path);
        return Outcome::Cancelled;
    }

    // ── pick ─────────────────────────────────────────────────────────────
    // AV1 wins outright when it is the only candidate, and otherwise only by
    // being SMALLER than an exact copy. The constraints are met either way, so
    // the tiebreak is bytes.
    let Some(chosen) = pick(&mut candidates) else {
        let _ = std::fs::remove_file(&copy_path);
        let _ = std::fs::remove_file(&av1_path);
        // Everything that came out was legal apart from its weight. That is not
        // a defect to report as one -- it is the answer, and it has a number.
        return match smallest_over {
            Some(smallest) => Outcome::StillTooLarge { smallest },
            None => Outcome::Failed("no candidate survived — this file needs a human".to_owned()),
        };
    };

    let treatment = match chosen.encode {
        Encode::Av1(crf) => Treatment::Av1 { crf },
        Encode::Copy => match av1_bytes {
            Some(av1) => Treatment::KeptOwnCodec {
                av1_ratio: av1 as f64 / source_bytes as f64,
            },
            None => Treatment::StreamCopy,
        },
    };

    for leftover in [&copy_path, &av1_path] {
        if *leftover != chosen.path {
            let _ = std::fs::remove_file(leftover);
        }
    }
    if let Err(error) = std::fs::rename(&chosen.path, output) {
        let _ = std::fs::remove_file(&chosen.path);
        return Outcome::Failed(format!("could not write the result: {error}"));
    }

    Outcome::Produced(Box::new(Produced {
        path: output.to_path_buf(),
        bytes: chosen.bytes,
        facts: chosen.facts,
        treatment,
        audio_dropped: before.has_audio,
        scaled_to: target.size,
        frame_rate_clamped: target.fps,
        trimmed_to: target.trim_to,
        opening_cut: trim.cut_opening,
        av1_cancelled,
    }))
}

struct Candidate {
    path: PathBuf,
    bytes: u64,
    facts: Facts,
    encode: Encode,
}

fn pick(candidates: &mut Vec<Candidate>) -> Option<Candidate> {
    let best = candidates
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| match (a.encode, b.encode) {
            // Equal bytes go to AV1: it plays through the mod's own dav1d on
            // every machine, while H.264/H.265 go through the game's Unity
            // player and Windows Media Foundation -- the path that gives some
            // players a black screen today.
            (Encode::Av1(_), Encode::Copy) if a.bytes == b.bytes => std::cmp::Ordering::Less,
            (Encode::Copy, Encode::Av1(_)) if a.bytes == b.bytes => std::cmp::Ordering::Greater,
            _ => a.bytes.cmp(&b.bytes),
        })
        .map(|(i, _)| i)?;
    Some(candidates.swap_remove(best))
}

/// Holds a candidate to the gate and to still being the same video.
struct Verify<'a> {
    before: &'a Facts,
    target: &'a Target,
}

impl Verify<'_> {
    /// `Ok` only for a file that is shippable and is still what was asked for.
    ///
    /// The gate check is the load-bearing one: a tool that can emit a file its
    /// own gate refuses has misled the charter, which is worse than not
    /// checking. The rest catch an encode that succeeded at the wrong job --
    /// scaled to the wrong size, or quietly not trimmed.
    ///
    /// Being over the size limit comes back as [`Rejected::TooLarge`] and
    /// everything else as [`Rejected::Broken`], because the caller does
    /// genuinely different things with them: one is a candidate that lost and
    /// the other is a defect.
    fn judge(&self, path: &Path, encode: Encode) -> Result<Candidate, Rejected> {
        let broken = |reason: String| Err(Rejected::Broken(reason));

        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => return broken(format!("the result could not be read back: {error}")),
        };
        let size = bytes.len() as u64;
        let Some(after) = gate::probe(&bytes) else {
            return broken("the result is not a readable MP4".to_owned());
        };

        if after.has_audio {
            return broken("the result still carries an audio track".to_owned());
        }

        let wanted_codec = match encode {
            Encode::Av1(_) => Some("av1"),
            Encode::Copy => self.before.codec,
        };
        if after.codec != wanted_codec {
            return broken(format!(
                "the result is {}, not {}",
                after.codec.unwrap_or("an unrecognised codec"),
                wanted_codec.unwrap_or("the source's codec"),
            ));
        }

        match gate::check(size, Some(&after)) {
            gate::Verdict::TooLarge { .. } => return Err(Rejected::TooLarge(size)),
            verdict if !verdict.passes() => {
                return broken(format!("the result would be refused: {}", verdict.code()));
            }
            _ => {}
        }

        let wanted_size = self
            .target
            .size
            .unwrap_or((self.before.width, self.before.height));
        if (after.width, after.height) != wanted_size {
            return broken(format!(
                "the result is {}x{}, expected {}x{}",
                after.width, after.height, wanted_size.0, wanted_size.1
            ));
        }

        match self.target.trim_to {
            Some(trim_to) => {
                // A trim lands on a packet boundary, so it can sit a fraction of
                // a second past the mark. A second of overshoot is not a
                // failure; silently not trimming at all is.
                if after.duration > trim_to + plan::TRIM_TOLERANCE {
                    return broken(format!(
                        "the trim did not take: {:.1}s against {trim_to:.1}s",
                        after.duration
                    ));
                }
            }
            None => {
                let drift = (after.duration - self.before.duration).abs() / self.before.duration;
                if drift > 0.02 {
                    return broken(format!(
                        "the length changed from {:.2}s to {:.2}s",
                        self.before.duration, after.duration
                    ));
                }
            }
        }

        Ok(Candidate {
            path: path.to_path_buf(),
            bytes: size,
            facts: after,
            encode,
        })
    }
}

impl Produced {
    pub fn ratio(&self, source_bytes: u64) -> f64 {
        self.bytes as f64 / source_bytes as f64
    }
}
