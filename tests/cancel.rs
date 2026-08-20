//! Stopping a run, and what is left behind when it stops.
//!
//! The cancel button is the only escape from a long encode, so "it kills the
//! process" is not enough to know -- what matters is that nothing half-written
//! survives, and that the caller can tell a cancellation from a failure. A
//! partial MP4 left on disk is worse than none, because it looks like a result.
//!
//! Run against the encoder Press would SHIP -- the embedded one where the build
//! carries it, PATH only in a development checkout. Skips rather than fails when
//! there is neither, so a checkout without ffmpeg still gets a green suite for
//! everything that does not need one.

use std::path::PathBuf;

use euterpe_press::ffmpeg::{Cancel, Ffmpeg, Ran};
use euterpe_press::plan::Trim;
use euterpe_press::standardise::{self, Outcome};

fn ffmpeg() -> Option<Ffmpeg> {
    // The encoder Press would SHIP, resolved exactly as the product resolves it
    // -- the embedded one where the build has it, PATH only in a development
    // checkout. Reaching for PATH here would test a fuller ffmpeg than the one
    // charters actually get.
    match euterpe_press::encoder::resolve() {
        Some(ffmpeg) => Some(ffmpeg),
        None => {
            eprintln!("skipping: no encoder embedded and none on PATH");
            None
        }
    }
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("euterpe-press-tests");
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir.join(name)
}

fn fixture(name: &str) -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// A token already set when the run starts stops it at the first progress
/// block, and the answer is `Cancelled` rather than the non-zero exit a killed
/// process actually produces. Reporting that exit as a failure would put an
/// ffmpeg error message in front of someone who pressed the button themselves.
#[test]
fn a_cancelled_run_is_not_reported_as_a_failure() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let output = scratch("cancelled.mp4");
    let _ = std::fs::remove_file(&output);

    // A real fixture rather than a synthetic `lavfi` source: the encoder Press
    // ships is built with only what Press needs, and lavfi is not part of that.
    // A test that requires a fuller ffmpeg than the product does is a test that
    // passes here and fails against the shipped binary.
    //
    // Its length does not matter. The token is set before the run starts, so the
    // first progress line ends it -- and if the encode finished before that line
    // was read, killing an already-exited process is harmless and the answer is
    // the same.
    let source = fixture("pixels_at_limit.mp4");
    let args: Vec<String> = [
        "ffmpeg",
        "-y",
        "-loglevel",
        "error",
        "-i",
        &source.to_string_lossy(),
        "-c:v",
        "libsvtav1",
        "-crf",
        "32",
        "-preset",
        "4",
        "-pix_fmt",
        "yuv420p",
        "-an",
        &output.to_string_lossy(),
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();

    let cancel = Cancel::new();
    cancel.request();

    let ran = ffmpeg
        .run(&args, Some(30.0), &cancel, |_| {})
        .expect("not a failure");
    assert_eq!(ran, Ran::Cancelled);

    let _ = std::fs::remove_file(&output);
}

/// Cancelling a file that MUST be re-encoded leaves nothing, because the encode
/// is the fix -- there is no compliant copy behind it to fall back on. The
/// window has to word its button for that, and this is the behaviour it words.
#[test]
fn cancelling_a_forced_reencode_leaves_nothing_on_disk() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let source = scratch("forced_source.mp4");
    std::fs::copy(fixture("vp9.mp4"), &source).expect("copy the fixture");
    let output = scratch("forced_out.mp4");
    let _ = std::fs::remove_file(&output);

    let cancel = Cancel::new();
    cancel.request();

    let outcome =
        standardise::standardise(&ffmpeg, &source, &output, Trim::default(), &cancel, |_| {});
    assert!(
        matches!(outcome, Outcome::Cancelled),
        "VP9 can only be fixed by re-encoding, so a cancel yields nothing: {outcome:?}"
    );
    assert!(!output.exists(), "a partial file must not be left behind");

    // And no candidate scratch files either -- they sit beside the output.
    for leftover in [".forced_out.copy.mp4", ".forced_out.av1.mp4"] {
        assert!(!scratch(leftover).exists(), "{leftover} was left behind");
    }

    let _ = std::fs::remove_file(&source);
}

/// Cancelling a file whose only problem is an audio track ships the stream copy:
/// it is already fully compliant, and all that was forfeited is the chance that
/// AV1 would have been smaller. The result says so, so the summary can.
#[test]
fn cancelling_the_comparison_still_ships_the_exact_copy() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let source = scratch("offered_source.mp4");
    std::fs::copy(fixture("audio_h264.mp4"), &source).expect("copy the fixture");
    let output = scratch("offered_out.mp4");
    let _ = std::fs::remove_file(&output);

    // Requested up front, so the copy is allowed to finish (it is checked per
    // progress block and the copy of a one-second file emits one) and the AV1
    // rung never starts.
    let cancel = Cancel::new();
    let outcome = standardise::standardise(
        &ffmpeg,
        &source,
        &output,
        Trim::default(),
        &cancel,
        |step| {
            // The moment the copy is being verified, ask to stop. That is the real
            // sequence: the user presses the button while the encode is queued.
            if matches!(step, standardise::Step::Verifying) {
                cancel.request();
            }
        },
    );

    match outcome {
        Outcome::Produced(produced) => {
            assert!(
                produced.av1_cancelled,
                "the comparison was cancelled and should say so"
            );
            assert!(produced.audio_dropped, "the audio track was the violation");
            assert_eq!(produced.treatment, standardise::Treatment::StreamCopy);
            assert!(output.exists(), "the compliant copy is the result");

            // And it is a file the server would take.
            let bytes = std::fs::read(&output).expect("read the result");
            assert!(euterpe_press::gate::judge(&bytes).1.passes());
        }
        other => panic!("the exact copy should have shipped: {other:?}"),
    }

    for leftover in [".offered_out.copy.mp4", ".offered_out.av1.mp4"] {
        assert!(!scratch(leftover).exists(), "{leftover} was left behind");
    }
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_file(&output);
}
