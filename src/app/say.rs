//! Every sentence the workbench and the bench put on screen.
//!
//! # Why the copy is its own module
//!
//! Not tidiness. Every wording bug this project has had lived in these
//! functions, and they were the hardest thing in it to look at: mixed in among
//! the painting, they could only be read by rendering a window. So the test that
//! reads every card in every language could not call them without a `Ui` -- and
//! rather than build one, it grew a hand-copy of the decisions instead, which
//! promptly drifted. It said "compressed" for every encode while the card had
//! already learnt to say "done" for one that grew.
//!
//! Nothing here takes a `Ui`. Given a bench and a language, each returns the
//! exact string that will be painted, which is a thing a test can simply read --
//! and `super::tests::card_lines` now calls these and nothing else.

use eframe::egui;

use super::*;

/// The headline, and the colour that carries its tone.
///
/// SEPARATE FROM THE PAINTING, so the test that reads every card in every
/// language reads the real decision instead of a hand-copy of it. It was a
/// hand-copy, and it had already drifted: the copy said "compressed" for every
/// encode while the card had learnt to say "done" for one that grew -- so the
/// test written to catch exactly that class of wording bug was blind to the two
/// branches added for it.
pub(super) fn headline(
    bench: &Bench,
    blocked: bool,
    requested: bool,
    lang: Lang,
) -> (&'static str, egui::Color32) {
    match (&bench.said, bench.encoded.is_some(), blocked) {
        (Some(Said::Failed(_)), ..) => (text::FAILED.get(lang), theme::ALARM),
        (Some(Said::StillTooLarge { .. }), ..) => (text::STILL_TOO_LARGE.get(lang), theme::NOTICE),
        // The file is fine; it is the crop that did not happen. Amber, not
        // green -- the charter asked for something and did not get it -- and
        // never "ready", which would be true and beside the point.
        (Some(Said::RequestDropped), ..) => (text::REQUEST_DROPPED.get(lang), theme::NOTICE),
        // Stopped by hand. Said plainly, because the alternative was the screen
        // the charter pressed Stop on, unchanged -- which reads as a Stop that
        // did not work.
        (Some(Said::Cancelled), ..) => (text::CANCELLED.get(lang), theme::INK),
        // "Compressed" is claimed only where it is true. A crop has to be
        // re-encoded, and a re-encode of an already-efficient file can come back
        // larger -- a fair outcome, and not one to put a congratulation on.
        (_, true, false) => (
            if bench
                .encoded
                .as_ref()
                .is_some_and(|d| d.bytes < bench.job.bytes)
            {
                text::BENCH_COMPRESSED.get(lang)
            } else {
                text::BENCH_APPLIED.get(lang)
            },
            theme::GOOD,
        ),
        // Neither good news nor bad, so neither green nor amber: a plain
        // statement that there is a press outstanding.
        (_, false, false) if requested => (text::BENCH_WAITING.get(lang), theme::INK),
        (_, _, false) => (text::BENCH_READY.get(lang), theme::GOOD),
        (_, _, true) => (text::NOT_READY.get(lang), theme::NOTICE),
    }
}

/// ONE line under the headline, and which one depends entirely on what the
/// headline just said.
pub(super) fn under_headline(
    bench: &Bench,
    pending: &[Pending],
    blocked: bool,
    lang: Lang,
) -> Option<String> {
    match &bench.said {
        Some(Said::Failed(reason)) => Some(reason.clone()),
        Some(Said::StillTooLarge { smallest }) => Some(
            text::STILL_TOO_LARGE_DETAIL
                .get(lang)
                .replace("{size}", &smallest.div_ceil(1024 * 1024).to_string())
                .replace(
                    "{limit}",
                    &limits::MAX_BYTES.div_ceil(1024 * 1024).to_string(),
                ),
        ),
        // Why the request was dropped, since the headline only says that it was.
        Some(Said::RequestDropped) => Some(text::REQUEST_DROPPED_DETAIL.get(lang).to_owned()),
        // AV1 was tried and lost. Press made that choice on the charter's
        // behalf, so it says what the choice was -- with the number where there
        // is one, and without inventing one where the encode was killed on its
        // way past the material's weight rather than allowed to finish. Saying
        // nothing at all left a minutes-long run looking like a no-op.
        Some(Said::MaterialWins { av1_was }) => Some(match av1_was {
            Some(was) => text::SAID_MATERIAL_WINS
                .get(lang)
                .replace("{size}", &megabytes(*was)),
            None => text::SAID_MATERIAL_WINS_UNMEASURED.get(lang).to_owned(),
        }),
        // The only thing worth saying about a stop is whether it cost anything.
        Some(Said::Cancelled) => Some(text::CANCELLED_DETAIL.get(lang).to_owned()),
        // What is wrong, in as few words as carry it. The numbers are on the
        // specification line below.
        _ if blocked => Some(wrong_line(pending, lang)),
        // Ready. The line answers "and what did my press do" -- where the
        // numbers below have not already answered it.
        _ => did_line(bench, lang),
    }
}

/// Just the name. On the bench there is exactly one file in play, and the
/// folder it came from is answered on the screen where it was chosen.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// One separator, everywhere. Three different spacings around the same dot is
/// most of what "间距很随意" was pointing at.
pub(super) const SEP: &str = " · ";

/// How the weight moved, in whichever direction it moved.
///
/// `None` only when it did not move at all. Stating the growth is not
/// self-flagellation: the two numbers are directly above it, so the charter
/// works it out either way, and a window that reports only its wins is a window
/// nobody believes about anything.
pub(super) fn delta_line(bench: &Bench, lang: Lang) -> Option<String> {
    let done = bench.encoded.as_ref()?;
    if bench.job.bytes == 0 || done.bytes == bench.job.bytes {
        return None;
    }
    let ratio = done.bytes as f64 / bench.job.bytes as f64;
    let (template, percent) = if done.bytes < bench.job.bytes {
        (&text::BENCH_SMALLER, (1.0 - ratio) * 100.0)
    } else {
        (&text::BENCH_BIGGER, (ratio - 1.0) * 100.0)
    };
    Some(
        template
            .get(lang)
            .replace("{percent}", &format!("{percent:.0}")),
    )
}

/// How much has been written, when anything has.
pub(super) fn written_line(lang: Lang, bytes: Option<u64>) -> Vec<String> {
    bytes
        .map(|bytes| {
            text::STEP_WRITTEN
                .get(lang)
                .replace("{size}", &megabytes(bytes))
                .replace("{limit}", &megabytes(limits::MAX_BYTES))
        })
        .into_iter()
        .collect()
}

/// EVERYTHING that happened to this file, in the past tense it happened in.
///
/// `None` only when literally nothing has been done and nothing was encoded --
/// the file arrived correct, which is itself worth saying.
///
/// This has been wrong twice, in opposite directions. It reported the lossless
/// pass ALONE, which put "nothing needed doing -- this is your own file"
/// directly above "compressed" and "140.1 MB -> 15.1 MB". The repair for that
/// borrowed the strings written for what WILL happen, which reads correctly in
/// Chinese and wrongly in English -- "Audio removed - scale it to 2956x1246" --
/// so the encode's work was dropped from the list instead, on the reasoning that
/// the tally and the specification line already covered it.
///
/// They do not. The tally covers the SIZE. The specification says the shape the
/// file ended up in, not that it was cropped rather than scaled to get there,
/// and it says nothing at all about an opening that was cut. So a run that
/// cropped the picture, cut its opening and re-encoded the whole thing reported
/// "去掉了音轨" and stopped. Five strings were the price of not saying that.
pub(super) fn did_line(bench: &Bench, lang: Lang) -> Option<String> {
    let mut did: Vec<String> = bench
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
        .collect();

    if let Some(done) = &bench.encoded {
        let before = bench.material.facts.picture();
        let after = done.facts.picture();

        // Cropping and scaling both change the frame size and are not the same
        // act: one throws picture away, the other keeps all of it at fewer
        // pixels. Which happened is knowable only from the choice that was made,
        // because the result looks identical from the outside.
        let cropped = bench.job.crop.and_then(|choice| choice.window(before));
        if let Some(window) = cropped {
            did.push(
                text::DID_CROP
                    .get(lang)
                    .replace("{width}", &window.width.to_string())
                    .replace("{height}", &window.height.to_string()),
            );
        }
        let mid = cropped.map_or(before, |w| (w.width, w.height));
        if after != mid {
            did.push(
                text::DID_SCALE
                    .get(lang)
                    .replace("{width}", &after.0.to_string())
                    .replace("{height}", &after.1.to_string()),
            );
        }

        let was_fps = crate::gate::probe::reported_fps(bench.material.facts.fps);
        let now_fps = crate::gate::probe::reported_fps(done.facts.fps);
        if now_fps != was_fps {
            did.push(
                text::DID_FPS
                    .get(lang)
                    .replace("{limit}", &now_fps.to_string()),
            );
        }

        if bench.job.trim.cut_opening > 0.0 {
            did.push(
                text::DID_OPENING
                    .get(lang)
                    .replace("{shift}", &clock(bench.job.trim.cut_opening)),
            );
        }

        // Not said when the source was AV1 already: "converted to AV1" about a
        // file that arrived as one is a sentence that describes nothing.
        if bench.material.facts.codec != Some("av1") {
            did.push(text::DID_CODEC.get(lang).to_owned());
        }
    }

    if !did.is_empty() {
        return Some(did.join(SEP));
    }
    bench
        .encoded
        .is_none()
        .then(|| text::DIFF_NOTHING.get(lang).to_owned())
}

/// What is wrong, in as few words as carry it.
pub(super) fn wrong_line(pending: &[Pending], lang: Lang) -> String {
    pending
        .iter()
        .filter(|item| item.is_a_problem())
        .map(|item| match item {
            Pending::Scale { .. } => text::SHORT_RESOLUTION.get(lang),
            Pending::FrameRate { .. } => text::SHORT_FPS.get(lang),
            Pending::Codec { .. } => text::SHORT_CODEC.get(lang),
            Pending::OpeningCut { .. } => text::SHORT_OPENING.get(lang),
            Pending::TooLarge { .. } => text::SHORT_SIZE.get(lang),
            // Never reached: a request is not a fault, and `wrong_line` is
            // built from the faults alone.
            Pending::Crop { .. } => text::SHORT_CROP.get(lang),
        })
        .collect::<Vec<_>>()
        .join(text::LIST_SEP.get(lang))
}

/// What the next press will do, in one sentence.
pub(super) fn will_line(pending: &[Pending], lang: Lang) -> String {
    let steps: Vec<String> = pending
        .iter()
        .map(|item| match item {
            Pending::Scale { to, .. } => text::WILL_SCALE
                .get(lang)
                .replace("{width}", &to.0.to_string())
                .replace("{height}", &to.1.to_string()),
            Pending::FrameRate { to, .. } => {
                text::WILL_FPS.get(lang).replace("{limit}", &to.to_string())
            }
            Pending::Codec { .. } => text::WILL_CODEC.get(lang).to_owned(),
            Pending::OpeningCut { seconds } => text::WILL_OPENING
                .get(lang)
                .replace("{shift}", &clock(*seconds)),
            Pending::TooLarge { limit, .. } => text::WILL_SIZE
                .get(lang)
                .replace("{limit}", &limit.div_ceil(1024 * 1024).to_string()),
            Pending::Crop { to } => text::WILL_CROP
                .get(lang)
                .replace("{width}", &to.0.to_string())
                .replace("{height}", &to.1.to_string()),
        })
        .collect();
    text::WILL_LEAD
        .get(lang)
        .replace("{steps}", &steps.join(text::LIST_SEP.get(lang)))
}

/// What the file IS, as data -- weight included, so the size is stated once in
/// one rounding. It used to appear as `375.4 MB` in the tally and `376 MB` in a
/// finding three lines apart, which is two answers to one question.
///
/// The codec appears ONCE too: the recipe said `AV1` and the shape said `av1`,
/// in two cases, on the same line.
pub(super) fn spec_line(bench: &Bench, facts: &Facts, bytes: u64) -> String {
    // The weight is here ONLY when nothing above has already stated it. With a
    // tally on screen it appears twice on one card, three lines apart.
    let mut parts = if bench.encoded.is_some() {
        vec![shape_only(facts)]
    } else {
        vec![megabytes(bytes), shape_only(facts)]
    };
    if let Some(done) = &bench.encoded {
        // The CRF, because it is the one parameter that explains the result
        // and there is no knob for it. NOT the attempt count: "×2" is internal
        // bookkeeping wearing a symbol nobody can decode.
        parts.push(format!("CRF {}", done.crf));
    }
    parts.join(SEP)
}

/// The DETERMINISTIC findings: everything whose answer follows from the file.
///
/// Size is deliberately absent. It is the one gate rule that is not settled by
/// looking at the header, and it belongs to Phase B along with the search that
/// resolves it.
pub(super) fn findings(
    facts: &Facts,
    crop: Option<crop::Window>,
    lang: Lang,
) -> Vec<(String, String)> {
    let mut out = Vec::new();
    // What the run will actually be handed. The pixel finding is about the
    // SOURCE -- that is what is wrong with the file on disk -- but its fix is
    // about the frame the encode will see, which the crop has already narrowed.
    let target = crate::plan::plan(facts, None, crop);
    let frame = crop.map_or(facts.picture(), |w| (w.width, w.height));

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
        let to = target.size.unwrap_or(frame);
        // A crop can be the whole fix. Where it is, the bullet says so instead
        // of naming a scale that is not going to happen.
        let fix = if target.size.is_none() && crop.is_some() {
            text::FIX_RESOLUTION_BY_CROP.get(lang)
        } else {
            text::FIX_RESOLUTION.get(lang)
        };
        out.push((
            text::FINDING_RESOLUTION
                .get(lang)
                .replace("{width}", &facts.width.to_string())
                .replace("{height}", &facts.height.to_string()),
            fix.replace("{width}", &to.0.to_string())
                .replace("{height}", &to.1.to_string()),
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

pub(super) fn shape(facts: &Facts, bytes: u64) -> String {
    format!("{}{SEP}{}", shape_only(facts), megabytes(bytes))
}

/// The same, without the weight -- for the places that state the weight
/// separately and larger.
pub(super) fn shape_only(facts: &Facts) -> String {
    format!(
        "{}{SEP}{}×{}{SEP}{:.0} fps{SEP}{}",
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

pub(super) fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

pub(super) fn clock(seconds: f64) -> String {
    let seconds = seconds.max(0.0);
    format!("{}:{:04.1}", (seconds / 60.0) as u64, seconds % 60.0)
}

/// The inspect screen lists the deterministic problems; the gate reports the
/// first problem of any kind. They must not contradict each other about the
/// ones they both cover -- a file the gate refuses for its frame rate with
/// nothing listed under it is a window disagreeing with the thing it exists to
/// report.
///
/// Size is excluded on both sides, because it is exactly what the inspect screen
/// declines to have an opinion about.
#[cfg(test)]
pub(super) fn verdict_matches_findings(facts: &Facts, lang: Lang) -> bool {
    // Under the limit by construction, so `check` can only complain about the
    // deterministic rules.
    let refused = !gate::check(0, Some(facts)).passes();
    refused == !findings(facts, None, lang).is_empty()
}
