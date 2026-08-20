//! A requested change never comes back heavier than what went into it.
//!
//! CRF is a QUALITY target, not a size one, so a source already compressed
//! harder than the house standard re-encodes LARGER even with fewer pixels in
//! it. That is fine where there is somewhere to fall back to. It is not fine for
//! a crop: the uncropped file is not what was asked for, so something has to
//! ship -- and "something" was being read as "anything".
//!
//! A 2.6 MB video, cropped, came back at 3.2 MB under a headline reading
//! "compressed". Cropping removes pixels and the audio track is already gone, so
//! there is always a quality at which the result weighs less; this is the test
//! that Press goes and finds it.

use euterpe_press::crop::{Choice, Shape};
use euterpe_press::ffmpeg::{Cancel, Ffmpeg};
use euterpe_press::plan::Trim;
use euterpe_press::workspace::Workspace;
use euterpe_press::{gate, phase_a, phase_b};

fn ffmpeg() -> Option<Ffmpeg> {
    match euterpe_press::encoder::resolve() {
        Some(ffmpeg) => Some(ffmpeg),
        None => {
            eprintln!("skipping: no encoder");
            None
        }
    }
}

#[test]
fn a_crop_never_makes_the_file_bigger() {
    let Some(ffmpeg) = ffmpeg() else { return };
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/portrait.mp4");
    let bytes = std::fs::read(&source).expect("fixture");
    let facts = gate::probe(&bytes).expect("readable");
    let source_bytes = bytes.len() as u64;
    drop(bytes);

    let work = Workspace::open().expect("a workspace");
    let cancel = Cancel::new();

    let settled = phase_a::run(
        &ffmpeg,
        &source,
        &facts,
        source_bytes,
        Trim::default(),
        &work,
        &cancel,
        |_| {},
    );
    let phase_a::Outcome::Settled(material) = settled else {
        panic!("the lossless pass produced nothing: {settled:?}");
    };

    let from = (material.facts.width, material.facts.height);
    let window = Choice::centred(Shape::Wide)
        .window(from)
        .expect("portrait is neither shape");

    // The aim itself has to be the one that carries a ceiling. Reaching here as
    // `Must` is the original bug: that path treats the gate's 80 MB as a budget
    // to spend, on a file that was never anywhere near it.
    let aim = phase_b::aim(&material.facts, material.bytes, 0.0, Some(window))
        .expect("a crop needs an encode");
    assert_eq!(
        aim,
        phase_b::Aim::Apply {
            no_larger_than: material.bytes
        },
        "a crop on a conforming file is a request, not a rescue"
    );

    let outcome = phase_b::run(
        &ffmpeg,
        &material,
        aim,
        0.0,
        Some(window),
        &work,
        &cancel,
        |_| {},
    );
    let phase_b::Outcome::Encoded(done) = outcome else {
        panic!("the crop had to be applied: {outcome:?}");
    };

    println!(
        "{from:?} {} B -> {}x{} {} B at CRF {}",
        material.bytes, done.facts.width, done.facts.height, done.bytes, done.crf
    );
    assert_eq!(
        (done.facts.width, done.facts.height),
        (window.width, window.height),
        "the crop must actually be in the result"
    );
    assert!(
        done.bytes <= material.bytes,
        "cropped away {:.0}% of the pixels and the file grew: {} B -> {} B",
        window.lost(from) * 100.0,
        material.bytes,
        done.bytes
    );
}
