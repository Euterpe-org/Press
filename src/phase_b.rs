//! Phase B: the fight with 80 MB.
//!
//! Everything Phase A decided is fixed. One thing is not: **how much quality to
//! spend**. That is not a deterministic question -- the answer depends on how
//! compressible this particular footage turns out to be, which nothing short of
//! encoding it will say -- so it gets its own phase, with its own screen, and it
//! may run more than once.
//!
//! # The rule
//!
//! > Press delivers the highest quality that satisfies this situation's size
//! > requirement, and never spends quality past [`CRF_BEST`].
//!
//! Which requirement applies is the whole of [`Aim`]:
//!
//! * [`Aim::Must`] -- the file is not deliverable as it stands, so the encode
//!   IS the fix. The requirement is the gate's 80 MB. Press aims just under it
//!   and takes the best CRF that fits, floored at [`CRF_BEST`].
//! * [`Aim::Apply`] -- the file is deliverable and the charter asked for a
//!   change to it anyway. There is no budget to spend, so the quality is the
//!   house standard; and the result must not weigh MORE than what went in,
//!   because a crop removes pixels and coming back larger is not a trade
//!   anybody asked for.
//! * [`Aim::Beat`] -- Phase A already produced a deliverable file, and it is
//!   LOSSLESS. Any encode is a loss of picture, taken deliberately in exchange
//!   for a smaller upload. The requirement is therefore "smaller than what we
//!   already have", and the quality is the house standard -- [`plan::CRF`] --
//!   because going finer makes the file bigger, which is the opposite of what
//!   was asked, and going coarser drops below the standard K set.
//!
//! # Why there is no ladder any more
//!
//! The old code walked a fixed ladder, 32 → 38 → 44, and stopped. That is three
//! guesses that ignore what the first one measured, and it could not go finer
//! than 32 at all -- a 94 MB source came out at 19.2 MB, spending a quarter of
//! the allowance and calling it done.
//!
//! Measured on our own recipe (`libsvtav1 preset 6`, CRF 14 through 50), every
//! +6 CRF multiplies the output by a strikingly steady factor:
//!
//! ```text
//!   CRF 14  7.61 MB      CRF 38  1.97 MB   ÷1.42
//!   CRF 20  5.40 MB  ÷1.41    CRF 44  1.39 MB   ÷1.42
//!   CRF 26  3.95 MB  ÷1.37    CRF 50  0.98 MB   ÷1.42
//!   CRF 32  2.79 MB  ÷1.41
//! ```
//!
//! So one measurement predicts the rest. [`crf_for`] inverts that law, and the
//! search usually needs a quarter of one encode plus one real one.
//!
//! # Why not ask ffmpeg to hit a size
//!
//! Because it cannot, closely enough to skip the check. Measured against a
//! 2.50 MB target on this recipe: capped CRF (`-crf` with `-maxrate`/`-bufsize`)
//! landed at 3.13 MB, **25% over**; single-pass VBR (`-b:v`) at 2.07 MB, **17%
//! under**. Either way the result has to be weighed afterwards -- and since it
//! has to be weighed anyway, CRF is the better instrument: it spends bits where
//! the picture needs them instead of spreading them evenly.

use std::cell::Cell;
use std::path::PathBuf;

use crate::crop;
use crate::ffmpeg::{Cancel, Ffmpeg, Progress, Ran, Watching};
use crate::gate::{Facts, Verdict, limits};
use crate::phase_a::Material;
use crate::plan::{self, Encode};
use crate::verify::{self, Expect};
use crate::workspace::Workspace;

/// The best quality Press will ever spend bytes on.
///
/// Not a technical bound -- SVT-AV1 goes to 0 -- but a judgement about the
/// content. A background video is dimmed, scrimmed and playing behind gameplay;
/// past CRF 20 the extra megabytes buy something nobody in a match will ever
/// see, and every one of them is a slower download for every player.
///
/// It cannot push a result over the limit: the floor only binds when the CRF
/// that would fill the allowance is already finer than 20, which is exactly the
/// case where 20 fits with room to spare.
pub const CRF_BEST: u32 = 20;

/// Mosaic, if that is what it takes.
///
/// SVT-AV1's own ceiling. A file that cannot pass the gate at 63 cannot pass at
/// all, and at that point the answer is not a worse encode -- it is that the
/// video is too long, which only the charter can fix.
pub const CRF_WORST: u32 = 63;

/// What every +6 CRF does to the size. See the module docs for the measurement.
const SIZE_PER_SIX_CRF: f64 = 0.709;

/// A fitted slope is clamped into this range before it is trusted.
///
/// Two measurements a couple of CRF apart on unusual footage can imply an
/// absurd exponent, and an absurd exponent sends the next guess somewhere
/// useless. The measured value is 0.709 and these bounds are wide around it.
const SLOPE_BOUNDS: (f64, f64) = (0.45, 0.95);

/// Aim this far under the requirement.
///
/// The law is good but not exact, and landing at 80.4 MB costs a whole extra
/// encode while landing at 76 MB costs nothing anybody can see.
const AIM_UNDER: f64 = 0.95;

/// How far into an encode its final size can be projected from.
///
/// Earlier is noise -- the first frames are a keyframe and a settling rate
/// control. Later saves less. A quarter is enough for the projection to be worth
/// acting on and cheap enough to throw away.
const PROJECT_AT: f64 = 0.25;

/// Below this, restarting costs more than the better CRF is worth. Three CRF is
/// about a 19% difference in size.
const WORTH_RESTARTING: u32 = 3;

/// Encodes that ran to completion.
const MOST_ENCODES: usize = 3;
/// Every invocation, including the cheap ones killed early.
const MOST_ATTEMPTS: usize = 6;

/// What Phase B is being asked to achieve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aim {
    /// The material is not deliverable. The encode is the fix, there is no
    /// falling back to what Phase A produced, and the gate's allowance is a
    /// BUDGET -- Press spends it down to [`CRF_BEST`] to get the best picture
    /// that fits.
    Must,
    /// The material is deliverable and the charter asked for a change to it --
    /// a crop, an opening cut. The encode has to happen and its result ships
    /// whatever it weighs, but there is no budget to spend: the file was never
    /// too big, so the quality is the house standard and nothing else.
    ///
    /// This exists because treating it as [`Aim::Must`] spent the allowance on
    /// a file that had never asked for it. A 2.6 MB video, cropped, came back at
    /// CRF 20 and 3.2 MB -- larger than the source, under a green headline
    /// reading "compressed".
    ///
    /// `no_larger_than` is the second half of that fix. CRF is a QUALITY target,
    /// not a size one, so a source already compressed harder than the house
    /// standard comes back bigger even with fewer pixels in it -- measured
    /// across eight real chart videos, results ran from 13% to 140% of the
    /// original. Cropping removes pixels and the audio track is already gone, so
    /// there is always a quality at which the result weighs less than what went
    /// in, and Press goes and finds it instead of shrugging.
    Apply { no_larger_than: u64 },
    /// The material is already deliverable and lossless. An encode only earns
    /// its place by being smaller than this.
    Beat { smaller_than: u64 },
}

/// Something the next encode will do, for the screen that asks whether to run it.
///
/// Two kinds, and the difference matters more than it looks. A resolution over
/// the budget is a PROBLEM: the file cannot be uploaded until it is fixed. A
/// crop, or a cut to the opening, is a REQUEST: the file is perfectly uploadable
/// without it and simply is not yet what the charter asked for. Calling both
/// "not ready to upload" says something untrue about the second.
#[derive(Debug, Clone, PartialEq)]
pub enum Pending {
    Scale {
        from: (u32, u32),
        to: (u32, u32),
    },
    FrameRate {
        from: i64,
        to: u32,
    },
    Codec {
        from: String,
    },
    TooLarge {
        now: u64,
        limit: u64,
    },
    /// The charter's own, both of these.
    OpeningCut {
        seconds: f64,
    },
    Crop {
        to: (u32, u32),
    },
}

impl Pending {
    /// Whether this stands between the file and the upload, as opposed to
    /// between the file and what was asked for.
    pub fn is_a_problem(&self) -> bool {
        !matches!(self, Self::OpeningCut { .. } | Self::Crop { .. })
    }
}

/// Where a Phase B run has got to.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Encoding {
        crf: u32,
        attempt: usize,
        progress: Option<Progress>,
    },
    /// Gave up on one CRF, having learnt enough to choose a better one.
    Recalibrating {
        from: u32,
        to: u32,
    },
    Verifying,
}

#[derive(Debug)]
pub struct Encoded {
    pub path: PathBuf,
    pub bytes: u64,
    pub facts: Facts,
    /// The quality actually used. Shown, because it is the one number that
    /// explains the result.
    pub crf: u32,
    /// How many encodes it took to get here.
    pub encodes: usize,
}

#[derive(Debug)]
pub enum Outcome {
    /// An encode won, and this is it.
    Encoded(Box<Encoded>),
    /// Every encode came back larger than the lossless material. AV1 lost, and
    /// what Phase A produced is what ships -- it is both smaller and exact.
    MaterialWins {
        av1_was: Option<u64>,
    },
    Cancelled,
    /// Even at [`CRF_WORST`] the result is over the limit. Not a defect: a video
    /// long enough that 80 MB cannot hold it at any quality worth shipping. The
    /// remaining lever is length, which Press will not pull on its own.
    StillTooLarge {
        smallest: u64,
    },
    Failed(String),
}

/// What is still wrong with a file, in the order it will be fixed.
pub fn pending(
    facts: &Facts,
    bytes: u64,
    opening_cut: f64,
    crop: Option<crop::Window>,
) -> Vec<Pending> {
    let target = plan::plan(facts, None, crop);
    let mut out = Vec::new();

    // First in the chain, so first in the list.
    if let Some(window) = crop {
        out.push(Pending::Crop {
            to: (window.width, window.height),
        });
    }

    if let Some(to) = target.size {
        out.push(Pending::Scale {
            from: (facts.width, facts.height),
            to,
        });
    }
    if let Some(to) = target.fps {
        out.push(Pending::FrameRate {
            from: crate::gate::probe::reported_fps(facts.fps),
            to,
        });
    }
    if !limits::codec_allowed(facts.codec) {
        out.push(Pending::Codec {
            from: if facts.raw_fourcc.is_empty() {
                "?".to_owned()
            } else {
                facts.raw_fourcc.clone()
            },
        });
    }
    if opening_cut > 0.0 {
        out.push(Pending::OpeningCut {
            seconds: opening_cut,
        });
    }
    if bytes > limits::MAX_BYTES {
        out.push(Pending::TooLarge {
            now: bytes,
            limit: limits::MAX_BYTES,
        });
    }
    out
}

/// What running Phase B on this material would be for, or `None` when it would
/// be for nothing.
///
/// The one case worth refusing outright: a file that already conforms and is
/// already AV1. Re-encoding AV1 to AV1 was measured at ~190% of the original for
/// no visible gain, so there is nothing to win and a generation to lose.
pub fn aim(facts: &Facts, bytes: u64, opening_cut: f64, crop: Option<crop::Window>) -> Option<Aim> {
    // A rescue dominates a request: a file that is over the limit AND has a
    // crop waiting is still a rescue, and the allowance is still a budget.
    let conforms = crate::gate::check(bytes, Some(facts)).passes();
    if !conforms {
        return Some(Aim::Must);
    }
    // Deliverable, and the charter asked for something anyway.
    if opening_cut > 0.0 || crop.is_some() {
        return Some(Aim::Apply {
            no_larger_than: bytes,
        });
    }
    if facts.codec == Some("av1") {
        return None;
    }
    Some(Aim::Beat {
        smaller_than: bytes,
    })
}

/// The CRF that would land on `want` bytes, given that `crf` produced `bytes`.
///
/// Inverts `size(c) = bytes · r^((c − crf) / 6)`. The result is clamped into
/// Press's own quality bounds, which is where "highest quality that fits, but
/// never past CRF 20" actually happens.
pub fn crf_for(crf: u32, bytes: u64, want: u64) -> u32 {
    crf_for_with(crf, bytes, want, SIZE_PER_SIX_CRF)
}

fn crf_for_with(crf: u32, bytes: u64, want: u64, ratio: f64) -> u32 {
    if bytes == 0 || want == 0 {
        return crf;
    }
    let steps = (want as f64 / bytes as f64).ln() / ratio.ln();
    let guess = f64::from(crf) + 6.0 * steps;
    guess
        .round()
        .clamp(f64::from(CRF_BEST), f64::from(CRF_WORST)) as u32
}

/// The same prediction, using the slope these two measurements actually showed
/// rather than the one the recipe usually shows.
fn crf_from_two(a: (u32, u64), b: (u32, u64), want: u64) -> u32 {
    let (crf_a, size_a) = a;
    let (crf_b, size_b) = b;
    if crf_a == crf_b || size_a == 0 || size_b == 0 || size_a == size_b {
        return crf_for(crf_b, size_b, want);
    }
    let span = f64::from(crf_b) - f64::from(crf_a);
    let ratio = (size_b as f64 / size_a as f64).powf(6.0 / span);
    if !ratio.is_finite() || ratio <= 0.0 {
        return crf_for(crf_b, size_b, want);
    }
    crf_for_with(
        crf_b,
        size_b,
        want,
        ratio.clamp(SLOPE_BOUNDS.0, SLOPE_BOUNDS.1),
    )
}

/// Why an encode was killed before it finished.
#[derive(Debug, Clone, Copy)]
enum Killed {
    /// The projection says a very different CRF is wanted.
    Recalibrate(u32),
    /// It has already passed the ceiling; nothing it does from here can help.
    Ceiling { projected: u64 },
}

/// Run the size contest.
#[allow(clippy::too_many_arguments)]
pub fn run(
    ffmpeg: &Ffmpeg,
    material: &Material,
    aim: Aim,
    opening_cut: f64,
    crop: Option<crop::Window>,
    work: &Workspace,
    cancel: &Cancel,
    mut on_step: impl FnMut(Step),
) -> Outcome {
    let facts = &material.facts;
    // Recomputed from the MATERIAL, and with no allowance: Phase A already cut
    // the tail, so anything left to do is picture work.
    let target = plan::plan(facts, None, crop);
    let seconds = (facts.duration - opening_cut).max(0.0);
    let expect = Expect {
        // Crop THEN scale, which is the order the filters run in. Reading
        // `size` alone expects the source's dimensions from a picture that has
        // just been cut smaller.
        size: target.frame_size((facts.width, facts.height)),
        codec: Some("av1"),
        seconds,
        tolerance: plan::TRIM_TOLERANCE,
    };

    // The ceiling can never be exceeded by a useful result; the aim is where
    // Press tries to land.
    let (ceiling, want) = match aim {
        Aim::Must => (
            limits::MAX_BYTES,
            (limits::MAX_BYTES as f64 * AIM_UNDER) as u64,
        ),
        // No budget to spend: the file was never too big. The ceiling is the
        // gate's, and it is only ever reached by escalating away from the house
        // standard, never by aiming at it.
        // TWO ceilings at once, and the lower one wins: the gate's, and the
        // weight of what went in. There is no budget to spend here -- the file
        // was never too big -- and coming back larger than the source is not an
        // outcome to accept just because the encode was unavoidable.
        Aim::Apply { no_larger_than } => {
            let ceiling = limits::MAX_BYTES.min(no_larger_than);
            (ceiling, (ceiling as f64 * AIM_UNDER) as u64)
        }
        Aim::Beat { smaller_than } => (smaller_than, smaller_than),
    };

    let input = material.path().to_string_lossy().into_owned();
    let mut crf = plan::CRF;
    let mut measured: Vec<(u32, u64)> = Vec::new();
    let mut best: Option<Encoded> = None;
    let mut smallest_over: Option<u64> = None;
    let mut encodes = 0usize;
    let mut attempts = 0usize;
    // The projection is allowed to restart the search once. After that there is
    // a real measurement, which is worth more than any projection.
    // Calibration is for spending a budget. Where there is none, the house
    // standard is the answer and there is nothing to calibrate towards.
    let calibrated = Cell::new(!matches!(aim, Aim::Must));

    while attempts < MOST_ATTEMPTS && encodes < MOST_ENCODES {
        if cancel.requested() {
            break;
        }
        attempts += 1;

        let output = work.file(&format!("fit-crf{crf}.mp4"));
        let cmd = if opening_cut > 0.0 {
            plan::command_with_opening_cut(
                &input,
                &output.to_string_lossy(),
                &target,
                crf,
                opening_cut,
                None,
            )
        } else {
            plan::command(&input, &output.to_string_lossy(), &target, Encode::Av1(crf))
        };

        let killed: Cell<Option<Killed>> = Cell::new(None);
        let attempt = encodes + 1;
        let ran = ffmpeg.run(&cmd, Some(seconds), cancel, |p| {
            on_step(Step::Encoding {
                crf,
                attempt,
                progress: Some(p),
            });

            // Already past the ceiling: no amount of remaining footage makes it
            // smaller, so every further second produces a file that will be
            // deleted.
            if p.bytes > ceiling {
                let projected = p
                    .fraction
                    .filter(|f| *f > 0.0)
                    .map_or(p.bytes, |f| (p.bytes as f64 / f) as u64);
                killed.set(Some(Killed::Ceiling { projected }));
                return Watching::Abandon;
            }

            // One calibration, a quarter of the way in.
            if !calibrated.get()
                && let Some(fraction) = p.fraction
                && fraction >= PROJECT_AT
            {
                calibrated.set(true);
                let projected = (p.bytes as f64 / fraction) as u64;
                let wanted = crf_for(crf, projected, want);
                if wanted.abs_diff(crf) >= WORTH_RESTARTING {
                    killed.set(Some(Killed::Recalibrate(wanted)));
                    return Watching::Abandon;
                }
            }
            Watching::Continue
        });

        match ran {
            Err(error) => {
                let _ = std::fs::remove_file(&output);
                return finish(best, Outcome::Failed(format!("the encode failed: {error}")));
            }
            Ok(Ran::Cancelled) => {
                let _ = std::fs::remove_file(&output);
                break;
            }
            Ok(Ran::Abandoned) => {
                let _ = std::fs::remove_file(&output);
                match killed.get() {
                    Some(Killed::Recalibrate(to)) => {
                        on_step(Step::Recalibrating { from: crf, to });
                        crf = to;
                        continue;
                    }
                    Some(Killed::Ceiling { projected }) => match aim {
                        // AV1 has lost. Every rung below this one is a worse
                        // picture in a race that is already over, and the
                        // material is both smaller and exact.
                        Aim::Beat { .. } => break,
                        // Over the gate's ceiling, so a coarser quality is the
                        // only way through -- for a rescue and for a request
                        // alike.
                        Aim::Must | Aim::Apply { .. } => {
                            if crf >= CRF_WORST {
                                smallest_over = Some(
                                    smallest_over.map_or(projected, |s: u64| s.min(projected)),
                                );
                                break;
                            }
                            let next = crf_for(crf, projected, want).max(crf + 1).min(CRF_WORST);
                            on_step(Step::Recalibrating {
                                from: crf,
                                to: next,
                            });
                            crf = next;
                            continue;
                        }
                    },
                    None => break,
                }
            }
            Ok(Ran::ToCompletion) => encodes += 1,
        }

        on_step(Step::Verifying);
        let checked = match verify::judge(&output, &expect) {
            Ok(checked) => checked,
            Err(broken) => {
                let _ = std::fs::remove_file(&output);
                return finish(best, Outcome::Failed(broken.0));
            }
        };
        measured.push((crf, checked.bytes));

        match checked.verdict {
            Verdict::Ok => {}
            Verdict::TooLarge { .. } => {
                smallest_over =
                    Some(smallest_over.map_or(checked.bytes, |s: u64| s.min(checked.bytes)));
                let _ = std::fs::remove_file(&output);
                if crf >= CRF_WORST {
                    break;
                }
                crf = next_crf(&measured, want).max(crf + 1).min(CRF_WORST);
                continue;
            }
            // An encode that produced the wrong shape. `verify` already checked
            // the frame size and the codec, so reaching here means the gate
            // disagrees about something Press thought it had handled.
            other => {
                let _ = std::fs::remove_file(&output);
                return finish(
                    best,
                    Outcome::Failed(format!("the result would be refused: {}", other.code())),
                );
            }
        }

        // It passes the gate. Whether it satisfies THIS AIM is a second
        // question, and the two aims answer it in opposite ways.
        //
        // `Beat` has somewhere to fall back to -- the lossless material is a
        // perfectly good file -- so an AV1 that is not smaller has simply lost.
        //
        // `Apply` does not. The material is not what was asked for, so the
        // encode has to produce something. But "produce something" is not the
        // same as "produce anything": a crop removes pixels and the audio track
        // is already gone, so a result LARGER than what went in is not a trade
        // anybody asked for. It goes coarser and tries again, which converges
        // because there is always a quality at which fewer pixels weigh less.
        if let Aim::Beat { smaller_than } = aim
            && checked.bytes >= smaller_than
        {
            let _ = std::fs::remove_file(&output);
            return Outcome::MaterialWins {
                av1_was: Some(checked.bytes),
            };
        }

        let winner = Encoded {
            path: output,
            bytes: checked.bytes,
            facts: checked.facts,
            crf,
            encodes,
        };

        if let Aim::Apply { no_larger_than } = aim
            && winner.bytes > no_larger_than
            && crf < CRF_WORST
            && encodes < MOST_ENCODES
        {
            // KEPT, not deleted. A valid result that is merely bigger than
            // wanted is still a valid result, and running out of budget having
            // thrown it away would lose the charter's crop entirely.
            if let Some(previous) = best.replace(winner) {
                let _ = std::fs::remove_file(&previous.path);
            }
            let next = next_crf(&measured, want).max(crf + 1).min(CRF_WORST);
            on_step(Step::Recalibrating {
                from: crf,
                to: next,
            });
            crf = next;
            continue;
        }

        // Is there quality left on the table? Only under `Must`, where the
        // allowance is a budget to spend rather than a line to stay under.
        if matches!(aim, Aim::Must) {
            let finer = next_crf(&measured, want);
            if finer + WORTH_RESTARTING <= crf && encodes < MOST_ENCODES {
                // Keep this one: if the finer attempt overshoots and the budget
                // runs out, it is still a correct answer.
                if let Some(previous) = best.replace(winner) {
                    let _ = std::fs::remove_file(&previous.path);
                }
                crf = finer;
                continue;
            }
        }

        if let Some(previous) = best.take() {
            let _ = std::fs::remove_file(&previous.path);
        }
        return Outcome::Encoded(Box::new(winner));
    }

    if cancel.requested() {
        return finish(best, Outcome::Cancelled);
    }
    match smallest_over {
        Some(smallest) => finish(best, Outcome::StillTooLarge { smallest }),
        None => finish(
            best,
            Outcome::MaterialWins {
                av1_was: measured.last().map(|(_, b)| *b),
            },
        ),
    }
}

/// A result already in hand beats whatever the search was about to say.
///
/// The search gives up for good reasons -- out of budget, cancelled, the next
/// rung overshot -- but if an earlier attempt already produced a file that fits,
/// that file is the answer and the reason for stopping is irrelevant.
fn finish(best: Option<Encoded>, otherwise: Outcome) -> Outcome {
    match best {
        Some(winner) => Outcome::Encoded(Box::new(winner)),
        None => otherwise,
    }
}

/// The next quality to try, given everything measured so far.
///
/// A two-point fit is only trusted BETWEEN its two points. Measured on the real
/// curve, a slope fitted from CRF 20 and 26 and then extrapolated out to 44 came
/// back two CRF wrong -- the local slope on that pair reads 0.731 against the
/// recipe's true 0.709, and twenty-four CRF of extrapolation turns that small
/// difference into a wasted encode.
///
/// So: interpolate where there is something to interpolate between, and
/// otherwise anchor on the CLOSEST measurement and use the law, which was
/// measured across the whole curve rather than across six CRF of it.
fn next_crf(measured: &[(u32, u64)], want: u64) -> u32 {
    if measured.is_empty() {
        return plan::CRF;
    }
    // A bracketing pair: one result above the target and one below it.
    let mut bracket: Option<((u32, u64), (u32, u64))> = None;
    for (i, above) in measured.iter().enumerate() {
        for below in &measured[i + 1..] {
            let (lo, hi) = if above.1 <= below.1 {
                (above, below)
            } else {
                (below, above)
            };
            if lo.1 <= want && want <= hi.1 {
                bracket = Some((*lo, *hi));
            }
        }
    }
    if let Some((lo, hi)) = bracket {
        return crf_from_two(lo, hi, want);
    }

    let anchor = measured
        .iter()
        .min_by_key(|(_, bytes)| bytes.abs_diff(want))
        .expect("not empty");
    crf_for(anchor.0, anchor.1, want)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Measured on our own recipe, `libsvtav1 preset 6 yuv420p`, over twenty
    /// seconds of a real 1080p source. This is the table the whole strategy is
    /// built on, so it is the table the strategy is tested against.
    const CURVE: [(u32, u64); 7] = [
        (14, 7_975_476),
        (20, 5_664_981),
        (26, 4_139_927),
        (32, 2_926_278),
        (38, 2_066_634),
        (44, 1_457_111),
        (50, 1_029_091),
    ];

    /// One measurement predicts the whole curve. If this drifts, the search
    /// stops converging in two encodes and starts guessing.
    #[test]
    fn one_measurement_predicts_the_rest_of_the_curve() {
        for (from_crf, from_bytes) in CURVE {
            for (want_crf, want_bytes) in CURVE {
                let predicted = crf_for(from_crf, from_bytes, want_bytes);
                let expected = want_crf.clamp(CRF_BEST, CRF_WORST);
                assert!(
                    predicted.abs_diff(expected) <= 1,
                    "from CRF {from_crf} aiming at {want_bytes} B: predicted {predicted}, \
                     the curve says {expected}"
                );
            }
        }
    }

    /// A fitted slope is trustworthy BETWEEN its two points and not beyond them.
    ///
    /// This is what `next_crf` exists to arrange, and it is worth stating as a
    /// test because the tempting version -- always use the two most recent
    /// measurements -- is wrong in a way that only shows up as an extra encode.
    #[test]
    fn a_fitted_slope_is_only_trusted_between_its_points() {
        for window in CURVE.windows(3) {
            let (lo, mid, hi) = (window[0], window[1], window[2]);
            // Interpolating: the pair brackets the target, and the fit is exact
            // to within a CRF.
            let predicted = crf_from_two(lo, hi, mid.1);
            assert!(
                predicted.abs_diff(mid.0) <= 1,
                "interpolating between {lo:?} and {hi:?} for {mid:?}: got {predicted}"
            );
        }
    }

    /// The whole point of the bracketing rule: given a pair that does NOT
    /// contain the target, `next_crf` falls back to the law instead of
    /// extrapolating a six-CRF slope across twenty-four.
    #[test]
    fn a_distant_target_uses_the_law_rather_than_a_local_slope() {
        // CRF 20 and 26, aiming at what CRF 44 produced -- far outside the pair.
        let measured = [CURVE[1], CURVE[2]];
        let (want_crf, want_bytes) = CURVE[5];
        assert_eq!(want_crf, 44);

        let extrapolated = crf_from_two(measured[0], measured[1], want_bytes);
        assert_eq!(
            extrapolated, 46,
            "the local slope really is wrong out here; if this changes, the \
             fallback below is no longer earning its place"
        );
        assert_eq!(next_crf(&measured, want_bytes), want_crf);
    }

    /// The complaint that started this: 94 MB in, 19.2 MB out at CRF 32, a
    /// quarter of the allowance spent and three quarters thrown away.
    ///
    /// The floor is what stops the correction going too far the other way --
    /// filling 80 MB would mean CRF 10, which is megabytes spent on a picture
    /// that is dimmed and behind gameplay.
    #[test]
    fn a_result_far_under_the_allowance_is_corrected_to_the_floor() {
        let want = (limits::MAX_BYTES as f64 * AIM_UNDER) as u64;
        let finer = crf_for(plan::CRF, 19_204_000, want);
        assert_eq!(finer, CRF_BEST, "the floor is what should stop this");
        assert!(
            finer + WORTH_RESTARTING <= plan::CRF,
            "and the gain has to be worth another encode"
        );
    }

    /// The other direction: something far over the limit gets a coarser CRF,
    /// and the ceiling holds.
    #[test]
    fn a_result_over_the_limit_is_corrected_upward() {
        let want = (limits::MAX_BYTES as f64 * AIM_UNDER) as u64;

        let over = crf_for(plan::CRF, 120 * 1024 * 1024, want);
        assert!(over > plan::CRF, "120 MB at CRF 32 needs a coarser CRF");

        // A video so long that no quality fits. The prediction saturates rather
        // than running off the end.
        let hopeless = crf_for(plan::CRF, 4 * 1024 * 1024 * 1024, want);
        assert_eq!(hopeless, CRF_WORST);
    }

    /// The floor can never be the reason a file misses the gate: it only binds
    /// when the CRF that would fill the allowance is already finer than it.
    #[test]
    fn the_quality_floor_cannot_push_a_result_over_the_limit() {
        let want = (limits::MAX_BYTES as f64 * AIM_UNDER) as u64;
        for bytes in [1_000_000_u64, 19_000_000, 60_000_000, 76_000_000] {
            let crf = crf_for(plan::CRF, bytes, want);
            if crf == CRF_BEST {
                // Predict the size at the floor from the same law.
                let steps = (f64::from(CRF_BEST) - f64::from(plan::CRF)) / 6.0;
                let at_floor = bytes as f64 * SIZE_PER_SIX_CRF.powf(steps);
                assert!(
                    at_floor <= limits::MAX_BYTES as f64,
                    "CRF {CRF_BEST} on a {bytes} B measurement projects {at_floor:.0} B, \
                     which is over the limit"
                );
            }
        }
    }
}
