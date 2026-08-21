//! Stopping a run, and what is left behind when it stops.
//!
//! The stop button is the only escape from a long encode, so "it kills the
//! process" is not enough to know -- what matters is that nothing half-written
//! survives, and that the caller can tell a cancellation from a failure. A
//! partial MP4 left on disk is worse than none, because it looks like a result.
//!
//! # What the phase split changed here
//!
//! Cancelling used to mean two different things depending on the file, and the
//! window had to word its button accordingly -- one population lost everything,
//! the other lost only the chance of a smaller file. Getting that prediction
//! wrong put "stop waiting, use what we have" in front of someone whose only
//! candidate was a 375 MB remux the gate refuses.
//!
//! It is now one thing. Phase A produces its material BEFORE Phase B starts, so
//! stopping Phase B always leaves that material behind. Only Phase A itself can
//! be stopped with nothing to show, and it takes seconds.
//!
//! Run against the encoder Press would SHIP -- the embedded one where the build
//! carries it, PATH only in a development checkout. Skips rather than fails when
//! there is neither, so a checkout without ffmpeg still gets a green suite for
//! everything that does not need one.

use std::path::PathBuf;

use euterpe_press::ffmpeg::{Cancel, Ffmpeg, Ran, Watching};
use euterpe_press::job::{Job, Request};
use euterpe_press::plan::Trim;
use euterpe_press::workspace::Workspace;
use euterpe_press::{gate, phase_a, phase_b};

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

fn facts_of(path: &std::path::Path) -> (euterpe_press::gate::Facts, u64) {
    let bytes = std::fs::read(path).expect("read the fixture");
    let facts = gate::probe(&bytes).expect("a readable fixture");
    (facts, bytes.len() as u64)
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
        .run(&args, Some(30.0), &cancel, |_| Watching::Continue)
        .expect("not a failure");
    assert_eq!(ran, Ran::Cancelled);

    let _ = std::fs::remove_file(&output);
}

/// Stopping Phase A leaves nothing, and leaves nothing BEHIND -- no partial
/// remux in the workspace pretending to be a result.
#[test]
fn cancelling_the_lossless_pass_writes_nothing() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let source = fixture("audio_h264.mp4");
    let (facts, bytes) = facts_of(&source);
    let work = Workspace::open().expect("a workspace");

    let cancel = Cancel::new();
    cancel.request();

    let outcome = phase_a::run(
        &ffmpeg,
        &Job {
            source: source.to_path_buf(),
            bytes,
            facts: facts.clone(),
            trim: Trim::default(),
            crop: None,
        },
        &work,
        &cancel,
        |_| {},
    );
    assert!(
        matches!(outcome, phase_a::Outcome::Cancelled),
        "a pass stopped before it began is a cancellation: {outcome:?}"
    );

    let left: Vec<_> = std::fs::read_dir(work.dir())
        .expect("read the workspace")
        .flatten()
        .map(|e| e.file_name())
        .collect();
    assert!(left.is_empty(), "a partial file was left behind: {left:?}");
}

/// Stopping Phase B always leaves the material, whatever the file was.
///
/// This is the whole point of the split, and it is checked on the population
/// that used to lose everything: a VP9 source can only be fixed by encoding, so
/// under the old design cancelling yielded nothing at all. Now Phase A has
/// already produced a remux -- still VP9, still not deliverable, but real -- and
/// the window can offer to go back to it rather than to nothing.
#[test]
fn cancelling_the_encode_leaves_the_material_behind() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let source = fixture("audio_vp9.mp4");
    let (facts, bytes) = facts_of(&source);
    let work = Workspace::open().expect("a workspace");

    let settled = phase_a::run(
        &ffmpeg,
        &Job {
            source: source.to_path_buf(),
            bytes,
            facts: facts.clone(),
            trim: Trim::default(),
            crop: None,
        },
        &work,
        &Cancel::new(),
        |_| {},
    );
    let phase_a::Outcome::Settled(material) = settled else {
        panic!("the lossless pass should have produced a material: {settled:?}");
    };
    assert!(material.path().exists(), "the material must be on disk");
    assert!(
        !material.is_the_source(),
        "the fixture has audio, so a copy was made"
    );

    // VP9 cannot be shipped, and the file is nowhere near 80 MB: an encode is\r
    // owed with no budget to spend and nothing to fall back on.\r
    let aim = phase_b::aim(&material.facts, material.bytes, &Request::default())
        .expect("an encode is needed");
    assert_eq!(aim, phase_b::Aim::Repair);

    let cancel = Cancel::new();
    cancel.request();
    let outcome = phase_b::run(
        &ffmpeg,
        &material,
        aim,
        &Request::default(),
        &work,
        &cancel,
        |_| {},
    );
    assert!(
        matches!(outcome, phase_b::Outcome::Cancelled),
        "expected a cancellation: {outcome:?}"
    );

    assert!(
        material.path().exists(),
        "cancelling the encode must not take the material with it"
    );
}

/// A file whose only problem is an audio track is fully deliverable after Phase
/// A alone. Phase B is then an OFFER -- make it smaller -- and stopping it costs
/// nothing but that chance.
#[test]
fn a_file_whose_only_problem_is_audio_is_deliverable_after_phase_a() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let source = fixture("audio_h264.mp4");
    let (facts, bytes) = facts_of(&source);
    let work = Workspace::open().expect("a workspace");

    let settled = phase_a::run(
        &ffmpeg,
        &Job {
            source: source.to_path_buf(),
            bytes,
            facts: facts.clone(),
            trim: Trim::default(),
            crop: None,
        },
        &work,
        &Cancel::new(),
        |_| {},
    );
    let phase_a::Outcome::Settled(material) = settled else {
        panic!("expected a material: {settled:?}");
    };

    let result = std::fs::read(material.path()).expect("read the material");
    assert!(
        gate::judge(&result).1.passes(),
        "the lossless pass alone should make this uploadable"
    );
    assert_eq!(
        phase_b::aim(&material.facts, material.bytes, &Request::default()),
        Some(phase_b::Aim::Beat),
        "with a deliverable file in hand, an encode only earns its place on size"
    );
}

/// Stop pressed DURING an encode, which is the only way it is ever pressed.
///
/// Every other test here cancels before the run starts, which short-circuits:
/// the loop sees the flag on its first check and returns without ever spawning
/// ffmpeg. That proves the flag is read. It does not prove the thing the button
/// actually promises -- that a running encoder is killed, and that the half
/// written MP4 it was in the middle of does not survive to be mistaken for a
/// result.
///
/// So this one lets the encode genuinely start, waits for ffmpeg to report real
/// progress, and only then asks it to stop.
#[test]
fn stopping_a_running_encode_leaves_nothing_half_written() {
    let Some(ffmpeg) = ffmpeg() else { return };

    let source = fixture("fps120.mp4");
    let (facts, bytes) = facts_of(&source);
    let work = Workspace::open().expect("a workspace");
    let cancel = Cancel::new();

    let settled = phase_a::run(
        &ffmpeg,
        &Job {
            source: source.to_path_buf(),
            bytes,
            facts,
            trim: Trim::default(),
            crop: None,
        },
        &work,
        &cancel,
        |_| {},
    );
    let phase_a::Outcome::Settled(material) = settled else {
        panic!("expected a material: {settled:?}");
    };
    let aim = phase_b::aim(&material.facts, material.bytes, &Request::default())
        .expect("120 fps needs an encode");

    // Cancelled from INSIDE the run, on a real progress report, so the encoder
    // is provably alive and mid-file when it is asked to stop.
    let saw_progress = std::cell::Cell::new(false);
    let outcome = phase_b::run(
        &ffmpeg,
        &material,
        aim,
        &Request::default(),
        &work,
        &cancel,
        |step| {
            if let phase_b::Step::Encoding {
                progress: Some(_), ..
            } = step
            {
                saw_progress.set(true);
                cancel.request();
            }
        },
    );

    assert!(
        saw_progress.get(),
        "the encode never reported progress, so nothing was actually interrupted"
    );
    assert!(
        matches!(outcome, phase_b::Outcome::Cancelled),
        "a stop is not a failure: {outcome:?}"
    );

    // The material is Phase A's and must survive; anything else in the
    // workspace is a candidate that was being written when the stop landed.
    let left: Vec<String> = std::fs::read_dir(work.dir())
        .expect("the workspace should still be there")
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| !material.path().ends_with(name.as_str()))
        .collect();
    assert!(
        left.is_empty(),
        "a stopped encode left files behind, which look like results: {left:?}"
    );
}
