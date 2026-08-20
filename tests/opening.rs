//! The opening moves when it was asked to, and never when it was not.
//!
//! K's red line for the server's round is that it only ever takes the tail:
//! shifting somebody's picture against the music is damage nobody requested.
//! Press honours that in the automatic path — `plan::command` has no `-ss` and
//! `tests/plan.rs` asserts it never will — and deliberately breaks it in one
//! place, when the charter drags the opening handle themselves.
//!
//! Both halves are claims about PIXELS, and the argument list only says what was
//! asked for. So these decode the frames and look.
//!
//! # Why not PSNR
//!
//! The server's runbook measures a first-frame PSNR by hand and it works there
//! because a human reads the number. As an automatic threshold it does not: its
//! own data has chart 233 scoring 22.0 dB against a source it is indistinguishable
//! from, because AV1 removed that source's film grain, while a genuinely
//! misaligned frame scores about 14. The two overlap, so any cutoff between them
//! rejects good encodes of grainy footage.
//!
//! What separates "same frame, re-encoded" from "different frame" is composition,
//! not fidelity — so the comparison is made on a heavily reduced version of each
//! frame, where grain and quantisation have averaged away and only the picture's
//! shape is left.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use euterpe_press::ffmpeg::{Cancel, Ffmpeg};
use euterpe_press::gate;
use euterpe_press::plan::Trim;
use euterpe_press::preview::{Frame, Preview};
use euterpe_press::standardise::{self, Outcome};

/// The reduced size frames are compared at. Small enough that compression noise
/// and grain are gone; large enough that two different moments of the same shot
/// still differ.
const COMPARE: u32 = 32;

fn ffmpeg() -> Option<Ffmpeg> {
    match euterpe_press::encoder::resolve() {
        Some(ffmpeg) => Some(ffmpeg),
        None => {
            eprintln!("skipping: no encoder embedded and none on PATH");
            None
        }
    }
}

fn fixture(name: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("euterpe-press-opening");
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.join(name)
}

/// One frame from a file, reduced. The viewfinder is already a frame extractor,
/// so there is no second decoding path to keep correct.
fn frame_at(ffmpeg: &Ffmpeg, path: &std::path::Path, seconds: f64) -> Frame {
    let preview = Preview::open(ffmpeg.binary(), path, COMPARE, COMPARE, 30.0);
    preview.seek(seconds);
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(frame) = preview.frame() {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("no frame from {} at {seconds}s", path.display());
}

/// Mean absolute difference per channel, 0 (identical) to 255 (inverted).
fn difference(a: &Frame, b: &Frame) -> f64 {
    assert_eq!(
        (a.width, a.height),
        (b.width, b.height),
        "frames must be the same size to compare"
    );
    let total: u64 = a
        .rgba
        .iter()
        .zip(b.rgba.iter())
        .map(|(x, y)| u64::from(x.abs_diff(*y)))
        .sum();
    total as f64 / a.rgba.len() as f64
}

/// How far apart in the source two frames are taken to calibrate "different".
const APART: f64 = 0.6;

/// There are no absolute thresholds here, on purpose.
///
/// A number chosen from one fixture is a number that will be wrong for the next
/// one — measured on these, the same frame twice comes back at exactly 0.00
/// while frames 0.6s apart sit between 8.5 and 14.7 depending on the reduction,
/// and a re-encoded frame lands somewhere in between. Any constant drawn through
/// that is a constant somebody will have to re-tune against real footage.
///
/// So every claim below is made RELATIVE to a known-different pair taken from
/// the same content in the same run: "closer to itself than to a frame that is
/// definitely not it". That holds whatever the material is.
fn much_closer_than(measured: f64, reference: f64) -> bool {
    measured < reference / 2.0
}

fn produce(
    ffmpeg: &Ffmpeg,
    source: &std::path::Path,
    out: &std::path::Path,
    trim: Trim,
) -> PathBuf {
    let _ = std::fs::remove_file(out);
    let cancel = Cancel::new();
    match standardise::standardise(ffmpeg, source, out, trim, &cancel, |_| {}) {
        Outcome::Produced(produced) => produced.path.clone(),
        other => panic!("expected a file: {other:?}"),
    }
}

/// The comparison itself has to be able to tell two frames apart, or every
/// assertion below passes for the wrong reason. This is the calibration.
#[test]
fn the_comparison_can_actually_tell_frames_apart() {
    let Some(ffmpeg) = ffmpeg() else { return };
    let source = fixture("plain_h264.mp4");

    let first = frame_at(&ffmpeg, &source, 0.0);
    let same = frame_at(&ffmpeg, &source, 0.0);
    let later = frame_at(&ffmpeg, &source, APART);

    let identical = difference(&first, &same);
    let moved = difference(&first, &later);

    // Decoding is deterministic, so the same moment twice is not merely close.
    assert_eq!(
        identical, 0.0,
        "the same moment twice must be the same bytes"
    );
    assert!(
        much_closer_than(identical, moved),
        "with no gap between these two cases nothing below means anything: \
         {identical:.2} against {moved:.2}"
    );
}

/// The automatic path keeps the first frame. This is the invariant the server's
/// round exists under, checked on pixels rather than on arguments.
#[test]
fn a_file_press_fixed_by_itself_still_starts_where_it_did() {
    let Some(ffmpeg) = ffmpeg() else { return };

    // Both populations: one fixed by remux, one that had to be re-encoded.
    for name in ["audio_h264.mp4", "fps120.mp4"] {
        let source = scratch(&format!("src_{name}"));
        std::fs::copy(fixture(name), &source).expect("copy the fixture");
        let out = produce(
            &ffmpeg,
            &source,
            &scratch(&format!("out_{name}")),
            Trim::default(),
        );

        let before = frame_at(&ffmpeg, &source, 0.0);
        let after = frame_at(&ffmpeg, &out, 0.0);

        let drift = difference(&before, &after);
        // Calibrated against this very file: a pair that is definitely not the
        // same picture.
        let definitely_different = difference(&before, &frame_at(&ffmpeg, &source, APART));
        assert!(
            much_closer_than(drift, definitely_different),
            "{name}: the opening moved without being asked to \
             ({drift:.2}, against {definitely_different:.2} for a frame that really did move)"
        );

        let _ = std::fs::remove_file(&source);
        let _ = std::fs::remove_file(&out);
    }
}

/// And when the charter DOES ask, it moves — to where they asked, not to the
/// nearest keyframe. A stream copy would land seconds away, which is why an
/// opening cut always re-encodes.
#[test]
fn an_opening_cut_starts_where_the_charter_put_it() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let source = scratch("cut_src.mp4");
    std::fs::copy(fixture("plain_h264.mp4"), &source).expect("copy the fixture");

    let cut = APART;
    let out = produce(
        &ffmpeg,
        &source,
        &scratch("cut_out.mp4"),
        Trim {
            cut_opening: cut,
            keep_until: None,
        },
    );

    let new_first = frame_at(&ffmpeg, &out, 0.0);
    let asked_for = frame_at(&ffmpeg, &source, cut);
    let old_first = frame_at(&ffmpeg, &source, 0.0);

    let landed = difference(&new_first, &asked_for);
    let moved = difference(&new_first, &old_first);

    // It starts at the frame the charter chose, and not at the one it used to
    // start at. Stated against each other, so "it landed on a keyframe two
    // seconds away" cannot pass by being merely small.
    assert!(
        much_closer_than(landed, moved),
        "the cut did not land where it was asked: {landed:.2} from the chosen frame, \
         {moved:.2} from the old one — a keyframe, most likely"
    );
    assert!(
        moved > 0.0,
        "the opening did not move at all, so the cut was ignored"
    );

    // The result is still a file the server takes: an authored trim is not a
    // licence to emit something that fails the gate.
    let bytes = std::fs::read(&out).expect("read the result");
    assert!(gate::judge(&bytes).1.passes());

    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&out);
}
