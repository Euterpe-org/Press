//! A source that is stored one way round and displayed the other.
//!
//! Phones and some editors write the picture into the file in one orientation
//! and a `tkhd` display matrix that turns it a quarter turn on playback. ffmpeg
//! honours that matrix on decode -- silently, by default -- so every frame it
//! hands back has the axes the other way round from what the header's sample
//! entry says.
//!
//! Press used to believe the sample entry. The consequence was not a wrong
//! number on a screen: the run paid for a whole AV1 encode, ffmpeg produced the
//! upright picture the file asked for, and `verify` then reported the correct
//! result as broken because it was expecting the other shape. With a crop
//! chosen it was worse -- the window was computed for the wrong axis and ffmpeg
//! refused the filter outright, so the run failed before it started.
//!
//! These decode the fixture and look, because the whole class of bug is Press
//! and ffmpeg disagreeing about what a frame is.

use std::path::{Path, PathBuf};

use euterpe_press::ffmpeg::Cancel;
use euterpe_press::job::{Job, Request};
use euterpe_press::plan::Trim;
use euterpe_press::workspace::Workspace;
use euterpe_press::{crop, gate, phase_a, phase_b};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn facts_of(name: &str) -> (gate::Facts, u64) {
    let bytes = std::fs::read(fixture(name)).expect("read the fixture");
    let facts = gate::probe(&bytes).expect("a readable fixture");
    (facts, bytes.len() as u64)
}

/// The header is read exactly as the server reads it, and the turn is read
/// alongside it rather than instead of it.
#[test]
fn the_coded_size_is_still_the_coded_size() {
    let (upright, _) = facts_of("plain_h264.mp4");
    let (turned, _) = facts_of("rotated.mp4");

    // Same bytes, same sample entry: the gate must not notice any difference,
    // because it rations a pixel COUNT and a count does not care which of its
    // two factors is called the width.
    assert_eq!(
        (turned.width, turned.height),
        (upright.width, upright.height)
    );
    assert_eq!(
        u64::from(turned.width) * u64::from(turned.height),
        u64::from(upright.width) * u64::from(upright.height)
    );

    assert!(!upright.quarter_turned, "no matrix, no turn");
    assert!(
        turned.quarter_turned,
        "a 90 degree matrix is a quarter turn"
    );

    // And the picture -- what ffmpeg will actually hand over -- is the other way
    // round.
    assert_eq!(upright.picture(), (upright.width, upright.height));
    assert_eq!(turned.picture(), (turned.height, turned.width));
}

/// Both phases, on a turned source, with the crop the shape of the DISPLAYED
/// picture calls for. The result has to come back a real file.
#[test]
fn a_turned_source_survives_a_crop_and_an_encode() {
    let Ok(ffmpeg) = euterpe_press::encoder::resolve() else {
        eprintln!("skipping: no encoder embedded and none on PATH");
        return;
    };

    let source = fixture("rotated.mp4");
    let (facts, bytes) = facts_of("rotated.mp4");
    let work = Workspace::open().expect("a workspace");
    let cancel = Cancel::new();

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
        &cancel,
        |_| {},
    );
    let phase_a::Outcome::Settled(material) = settled else {
        panic!("the lossless pass produced nothing: {settled:?}");
    };

    // 180x320 displayed: portrait, so neither shape a player's screen is, and
    // the crop is offered. Computed on the PICTURE -- computed on the coded
    // 320x180 it would come back landscape and ffmpeg would refuse it.
    let picture = material.facts.picture();
    assert!(
        crop::is_unusual(picture),
        "a portrait clip should be offered a crop: {picture:?}"
    );
    let window = crop::Choice::centred(crop::Shape::Wide)
        .window(picture)
        .expect("portrait is neither shape");
    assert!(
        window.width <= picture.0 && window.height <= picture.1,
        "the window must fit the picture: {window:?} in {picture:?}"
    );

    let asked = Request {
        opening_cut: 0.0,
        crop: Some(window),
    };
    let aim =
        phase_b::aim(&material.facts, material.bytes, &asked).expect("a crop needs an encode");
    let outcome = phase_b::run(&ffmpeg, &material, aim, &asked, &work, &cancel, |_| {});
    let phase_b::Outcome::Encoded(done) = outcome else {
        panic!("a turned source must still encode: {outcome:?}");
    };

    // Measured on the result, not on the arguments: this is the assertion that
    // verify used to fail, and it fails again the moment Press goes back to
    // reasoning about the coded size.
    assert_eq!(
        done.facts.picture(),
        (window.width, window.height),
        "the encode must produce the window that was asked for"
    );
    let written = std::fs::read(&done.path).expect("read the result");
    assert!(
        gate::judge(&written).1.passes(),
        "the result of a turned source still has to pass the gate"
    );
}
