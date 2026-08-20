//! The port is held to the server's own answers, not to my reading of them.
//!
//! `tests/fixtures/golden.json` is written by `tools/golden.py`, which imports
//! `Euterpe.Web/backend/app/domain/video_probe.py` and records what it says
//! about every fixture. This asserts the Rust agrees, field for field.
//!
//! A charter told "this file is fine" by Press and "refused" by the server has
//! been actively misled -- worse off than with no tool at all. That is the whole
//! reason this file exists.

use std::path::{Path, PathBuf};

use euterpe_press::gate::{self, Verdict, limits};
use serde::Deserialize;

#[derive(Deserialize)]
struct Golden {
    constants: Constants,
    fixtures: Vec<Fixture>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Constants {
    VIDEO_MAX_BYTES: u64,
    VIDEO_MAX_WIDTH: u32,
    VIDEO_MAX_HEIGHT: u32,
    VIDEO_MAX_PIXELS: u64,
    VIDEO_MAX_FPS: u32,
    VIDEO_ALLOWED_CODECS: Vec<String>,
    VIDEO_OVERRUN_ALLOWANCE_SECONDS: f64,
}

#[derive(Deserialize)]
struct Fixture {
    file: String,
    size: u64,
    facts: Option<GoldenFacts>,
    verdict: Option<GoldenVerdict>,
}

#[derive(Deserialize)]
struct GoldenFacts {
    codec: Option<String>,
    raw_fourcc: String,
    width: u32,
    height: u32,
    fps: f64,
    duration: f64,
    has_audio: bool,
}

#[derive(Deserialize)]
struct GoldenVerdict {
    code: String,
    params: serde_json::Value,
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn golden() -> Golden {
    let path = fixtures_dir().join("golden.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is missing ({e}). Run tools/golden.py.", path.display()));
    serde_json::from_str(&raw).expect("golden.json is not valid JSON")
}

/// The params the server interpolates into the sentence, rebuilt from the typed
/// verdict. Compared as JSON so a missing or extra key fails rather than being
/// quietly ignored.
fn params_of(verdict: &Verdict) -> serde_json::Value {
    use serde_json::json;
    match verdict {
        Verdict::Ok => json!(null),
        Verdict::TooLarge { size_mb, limit_mb } => json!({"size": size_mb, "limit": limit_mb}),
        Verdict::Invalid => json!({}),
        Verdict::FormatUnsupported { codec } => json!({"codec": codec}),
        Verdict::ResolutionTooHigh { width, height, max_width, max_height } => json!({
            "width": width, "height": height,
            "maxWidth": max_width, "maxHeight": max_height,
        }),
        Verdict::FpsTooHigh { fps, limit } => json!({"fps": fps, "limit": limit}),
        Verdict::HasAudio => json!({}),
    }
}

/// A threshold can move without any fixture crossing it, and that still has to
/// fail. `tools/golden.py --check` catches it at the source; this catches it in
/// `cargo test` on a machine with no Python.
#[test]
fn the_limits_are_the_servers() {
    let c = golden().constants;
    assert_eq!(limits::MAX_BYTES, c.VIDEO_MAX_BYTES, "MAX_BYTES");
    assert_eq!(limits::MAX_WIDTH, c.VIDEO_MAX_WIDTH, "MAX_WIDTH");
    assert_eq!(limits::MAX_HEIGHT, c.VIDEO_MAX_HEIGHT, "MAX_HEIGHT");
    assert_eq!(limits::MAX_PIXELS, c.VIDEO_MAX_PIXELS, "MAX_PIXELS");
    assert_eq!(limits::MAX_FPS, c.VIDEO_MAX_FPS, "MAX_FPS");
    assert_eq!(
        limits::OVERRUN_ALLOWANCE_SECONDS, c.VIDEO_OVERRUN_ALLOWANCE_SECONDS,
        "OVERRUN_ALLOWANCE_SECONDS"
    );

    let mut ours: Vec<&str> = limits::ALLOWED_CODECS.to_vec();
    ours.sort_unstable();
    assert_eq!(ours, c.VIDEO_ALLOWED_CODECS, "ALLOWED_CODECS");
}

#[test]
fn every_fixture_is_read_exactly_as_the_server_reads_it() {
    let golden = golden();
    assert!(!golden.fixtures.is_empty(), "no fixtures recorded");

    for fixture in &golden.fixtures {
        let path = fixtures_dir().join(&fixture.file);
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("{}: {e}. Run tools/make_fixtures.ps1.", path.display()));
        assert_eq!(bytes.len() as u64, fixture.size, "{}: size on disk", fixture.file);

        let (facts, verdict) = gate::judge(&bytes);
        let name = &fixture.file;

        match (&facts, &fixture.facts) {
            (None, None) => {}
            (Some(got), None) => panic!("{name}: server reads it as unreadable, we read {got:?}"),
            (None, Some(_)) => panic!("{name}: server reads it, we call it unreadable"),
            (Some(got), Some(want)) => {
                assert_eq!(got.codec.map(str::to_owned), want.codec, "{name}: codec");
                assert_eq!(got.raw_fourcc, want.raw_fourcc, "{name}: raw_fourcc");
                assert_eq!(got.width, want.width, "{name}: width");
                assert_eq!(got.height, want.height, "{name}: height");
                assert_eq!(got.has_audio, want.has_audio, "{name}: has_audio");
                // The golden is rounded to six places, which is far finer than
                // any real parsing difference and coarse enough that the last
                // bit of an f64 division cannot fail the test on its own.
                assert_eq!(round6(got.fps), want.fps, "{name}: fps");
                assert_eq!(round6(got.duration), want.duration, "{name}: duration");
            }
        }

        let (want_code, want_params) = match &fixture.verdict {
            None => ("OK", serde_json::json!(null)),
            Some(v) => (v.code.as_str(), v.params.clone()),
        };
        assert_eq!(verdict.code(), want_code, "{name}: verdict code");
        assert_eq!(params_of(&verdict), want_params, "{name}: verdict params");
    }
}

fn round6(x: f64) -> f64 {
    (x * 1e6).round() / 1e6
}

/// Resolution is rationed by PIXEL COUNT, and the limit is inclusive. Three
/// charts in the live library are 1080x1920 portrait, which a height rule would
/// refuse while passing the identical 1920x1080 -- so the shape of the frame is
/// asserted not to matter, only its area.
#[test]
fn the_pixel_budget_is_an_area_and_its_edge_is_inclusive() {
    let verdict = |file: &str| {
        let bytes = std::fs::read(fixtures_dir().join(file)).expect("fixture");
        gate::judge(&bytes).1
    };

    assert!(verdict("pixels_at_limit.mp4").passes(), "2560x1440 is exactly the budget");
    assert_eq!(
        verdict("pixels_over.mp4"),
        Verdict::ResolutionTooHigh { width: 2560, height: 1444, max_width: 2560, max_height: 1440 },
        "four rows past the budget is over it"
    );

    // 180x320 is the same area as 320x180 and just as acceptable.
    let portrait = std::fs::read(fixtures_dir().join("portrait.mp4")).expect("fixture");
    let facts = gate::probe(&portrait).expect("parses");
    assert!(facts.height > facts.width, "the fixture should be taller than it is wide");
    assert!(gate::check(portrait.len() as u64, Some(&facts)).passes());
}

/// `VIDEO_TOO_LARGE` has no fixture: an 80 MB file does not belong in a
/// repository. Size is judged before the header is even read, so it can be
/// exercised with any readable file and a claimed length.
#[test]
fn size_is_judged_before_anything_is_parsed() {
    let bytes = std::fs::read(fixtures_dir().join("plain_h264.mp4")).expect("fixture");
    let facts = gate::probe(&bytes);
    assert!(facts.is_some(), "the fixture should parse");

    assert!(gate::check(limits::MAX_BYTES, facts.as_ref()).passes(), "exactly at the limit passes");

    let over = gate::check(limits::MAX_BYTES + 1, facts.as_ref());
    assert_eq!(
        over,
        Verdict::TooLarge { size_mb: 81, limit_mb: 80 },
        "one byte over must round UP to 81, or the sentence reads '80 MB, limit 80 MB'"
    );

    // Unreadable AND oversize reports the size: it is the cheaper fact and the
    // one the user can act on without a decoder.
    assert_eq!(gate::check(limits::MAX_BYTES + 1, None).code(), "VIDEO_TOO_LARGE");
}

/// The media header may CORRECT the sample table, never overrule it -- and when
/// the correction lands on an exact half tick, it rounds half UP.
///
/// Half-up is not this port's choice: Euterpe.Web unified on it (`d5eeba36`)
/// precisely so that every implementation of the gate can use its own language's
/// rounding. Python spells it `floor(x + 0.5)` because `round()` there sends
/// halves to EVEN; JavaScript's `Math.round` and Rust's `f64::round` already do
/// the right thing. Getting it wrong here shifts the reported frame rate by
/// 0.0019 -- six orders of magnitude above any float tolerance, so this fails
/// loudly rather than drifting.
///
/// The three fixtures are hand-assembled (`tools/synth.py`) because no muxer
/// writes a header that contradicts its own sample table, and that contradiction
/// is the entire subject.
#[test]
fn the_media_header_corrects_the_sample_table_but_cannot_overrule_it() {
    let facts = |file: &str| {
        let bytes = std::fs::read(fixtures_dir().join(file)).expect("fixture");
        gate::probe(&bytes).expect("parses")
    };

    // 306 samples x 33 ticks = 10098, and 10098 * 1.25 is exactly 12622.5.
    // Half-up caps at 12623; half-to-even would cap at 12622 and report
    // 24.243385 instead.
    assert_eq!(round6(facts("synth_stretch_half.mp4").fps), 24.241464, "half rounds UP");

    // Within the 1.25x allowance the header simply wins: live chart 233 reads
    // 30.31 fps from its table alone and 30.01 once the header is consulted.
    assert_eq!(round6(facts("synth_stretch_corrects.mp4").duration), 10.298);

    // Past it, the cap is what is used. A header claiming three times its own
    // samples' duration would otherwise divide the frame rate and wave a
    // 120 fps file straight through.
    let lies = facts("synth_stretch_lies.mp4");
    assert_eq!(round6(lies.duration), 12.623, "capped at 1.25x, not the header's 30.0s");
    assert_eq!(round6(lies.fps), round6(facts("synth_stretch_half.mp4").fps));
}

/// The structural problems come first, each ahead of an audio track that would
/// otherwise shadow them. This is the bug Press found in the server on
/// 2026-08-20 (Euterpe.Web `70a9fa8f`): the audio test sat above the frame-rate
/// test, so a 120 fps file with audio cost the charter two round trips.
#[test]
fn a_structural_problem_is_named_before_the_audio_track() {
    for (file, expected) in [
        ("audio_vp9.mp4", "VIDEO_FORMAT_UNSUPPORTED"),
        ("audio_pixels_over.mp4", "VIDEO_RESOLUTION_TOO_HIGH"),
        ("audio_fps120.mp4", "VIDEO_FPS_TOO_HIGH"),
    ] {
        let bytes = std::fs::read(fixtures_dir().join(file)).expect("fixture");
        let (facts, verdict) = gate::judge(&bytes);
        assert!(facts.expect("parses").has_audio, "{file} should carry audio");
        assert_eq!(verdict.code(), expected, "{file}");
    }

    // And not over-corrected: when the audio IS the only problem, say so.
    let bytes = std::fs::read(fixtures_dir().join("audio_h264.mp4")).expect("fixture");
    assert_eq!(gate::judge(&bytes).1.code(), "VIDEO_HAS_AUDIO");
}
