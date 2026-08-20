//! The transcode plan, and the invariants of the command it produces.
//!
//! Two halves, pinned differently, on the recipe owner's own division
//! (Euterpe.Web, 2026-08-20):
//!
//! * **`plan()` is law.** What it returns -- which population a file falls into,
//!   what has to change about it -- is exactly the kind of thing that must not
//!   drift between implementations, so every scenario the server plans is
//!   asserted here, field for field, from a recorded snapshot.
//!
//! * **The exact argument list is not.** CRF, preset and scaler are TUNING
//!   KNOBS: they moved twice in the week this port was written (a savings round
//!   became a standardisation round, then AV1 had to start competing on size).
//!   Snapshotting a whole command line would turn every legitimate adjustment
//!   into a red build here, and a test that cries wolf is a test that gets
//!   ignored -- which costs more than the drift it was watching for.
//!
//! So what is asserted about the command is what holds under ANY tuning: the
//! trim is one-sided, the audio always goes, the copy path never touches the
//! picture, the re-encode path always clamps what it was asked to clamp.

use euterpe_press::gate::Facts;
use euterpe_press::plan::{self, Encode, Target};
use serde::Deserialize;

#[derive(Deserialize)]
struct Recipe {
    constants: Constants,
    scenarios: Vec<Scenario>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct Constants {
    CRF: u32,
    PRESET: u32,
    CRF_ESCALATION: Vec<u32>,
    LOSSLESS_GROWTH_MARGIN: f64,
    TRIM_TOLERANCE: f64,
}

#[derive(Deserialize)]
struct Scenario {
    name: String,
    facts: ScenarioFacts,
    allowance: Option<f64>,
    target: ScenarioTarget,
}

#[derive(Deserialize)]
struct ScenarioFacts {
    /// The sample-entry FOURCC, which is what a real file carries. The codec is
    /// derived from it below rather than read from the snapshot, so a scenario
    /// cannot name a codec no file produces.
    raw_fourcc: String,
    width: u32,
    height: u32,
    fps: f64,
    duration: f64,
    has_audio: bool,
}

#[derive(Deserialize)]
struct ScenarioTarget {
    size: Option<Vec<u32>>,
    fps: Option<u32>,
    trim_to: Option<f64>,
    must_reencode: bool,
    may_try_av1: bool,
    is_noop: bool,
}

fn recipe() -> Recipe {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/recipe.json");
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} is missing ({e})", path.display()));
    serde_json::from_str(&raw).expect("recipe.json is not valid JSON")
}

/// Facts built the way a real file produces them: the codec comes from the
/// FOURCC through the probe's own table, never from a name in the snapshot.
///
/// Both this port and Euterpe.Web's own tests once carried a scenario with
/// `codec = "vp9"`. Every assertion passed -- `None` and an unrecognised name
/// are refused for the same reason -- which is exactly why it was worth fixing
/// rather than shrugging at: a test standing on an impossible input reports
/// coverage it does not have. Both were fixed by removing the entry point.
fn facts_of(f: &ScenarioFacts) -> Facts {
    let fourcc: [u8; 4] = f
        .raw_fourcc
        .as_bytes()
        .try_into()
        .unwrap_or_else(|_| panic!("a FOURCC is four bytes, got {:?}", f.raw_fourcc));

    Facts {
        codec: euterpe_press::gate::probe::codec_of(&fourcc),
        raw_fourcc: f.raw_fourcc.clone(),
        width: f.width,
        height: f.height,
        fps: f.fps,
        duration: f.duration,
        has_audio: f.has_audio,
    }
}

/// Catches an edit to a constant HERE that was made without looking at what the
/// server currently uses.
///
/// It cannot catch the server tuning its own values, and is not meant to: the
/// snapshot only moves when `tools/recipe_snapshot.py` is re-run by hand. That
/// is the point -- a live check would go red on somebody else's ordinary
/// adjustment. Following a tuning change is a decision, and it arrives the way
/// the gate's constants do, by being told.
///
/// It is worth following. A file Press produces at a different CRF is a file the
/// server's round will re-encode later, which is the `video_holders_stale` debt
/// that using the same recipe exists to avoid.
#[test]
fn the_recipe_constants_match_what_was_last_recorded() {
    let c = recipe().constants;
    assert_eq!(plan::CRF, c.CRF, "CRF");
    assert_eq!(plan::PRESET, c.PRESET, "preset");
    assert_eq!(
        plan::CRF_ESCALATION.to_vec(),
        c.CRF_ESCALATION,
        "CRF escalation ladder"
    );
    assert_eq!(
        plan::LOSSLESS_GROWTH_MARGIN,
        c.LOSSLESS_GROWTH_MARGIN,
        "growth margin"
    );
    assert_eq!(plan::TRIM_TOLERANCE, c.TRIM_TOLERANCE, "trim tolerance");
}

#[test]
fn every_scenario_is_planned_the_way_the_server_plans_it() {
    let recipe = recipe();
    assert!(!recipe.scenarios.is_empty(), "no scenarios recorded");

    for scenario in &recipe.scenarios {
        let name = &scenario.name;
        let got = plan::plan(&facts_of(&scenario.facts), scenario.allowance);
        let want = &scenario.target;

        assert_eq!(
            got.size.map(|(w, h)| vec![w, h]),
            want.size,
            "{name}: frame size"
        );
        assert_eq!(got.fps, want.fps, "{name}: frame rate");
        assert_eq!(got.trim_to, want.trim_to, "{name}: trim");
        assert_eq!(
            got.must_reencode, want.must_reencode,
            "{name}: re-encode forced"
        );
        assert_eq!(got.may_try_av1, want.may_try_av1, "{name}: AV1 offered");
        assert_eq!(got.is_noop(), want.is_noop, "{name}: is_noop");
    }
}

// ── invariants of the command, true under any tuning ─────────────────────

fn index_of(cmd: &[String], arg: &str) -> Option<usize> {
    cmd.iter().position(|a| a == arg)
}

fn has(cmd: &[String], arg: &str) -> bool {
    index_of(cmd, arg).is_some()
}

/// Every command the planner can be asked to build, across every recorded
/// scenario and both encoders.
fn all_commands() -> Vec<(String, Target, Encode, Vec<String>)> {
    let mut out = Vec::new();
    for scenario in &recipe().scenarios {
        let target = plan::plan(&facts_of(&scenario.facts), scenario.allowance);
        for encode in [Encode::Copy, Encode::Av1(plan::CRF), Encode::Av1(44)] {
            let cmd = plan::command("IN.mp4", "OUT.mp4", &target, encode);
            out.push((scenario.name.clone(), target.clone(), encode, cmd));
        }
    }
    out
}

/// The trim cuts the TAIL and never the opening -- K's red line.
///
/// Two halves, and both are structural rather than conventional. `-ss` does not
/// appear anywhere, so there is nothing that could seek; and `-t` sits AFTER
/// `-i`, which makes it an output-side duration rather than an input-side seek.
/// Written the other way round the same flag would move the opening.
///
/// Verified on the server's round by measurement, not by reading arguments:
/// across twelve trimmed charts the first frame came back bit-identical ten
/// times, and at 69.0 / 56.6 dB for the two that were re-encoded. A shifted
/// opening collapses that to roughly 14 dB.
#[test]
fn the_trim_can_only_ever_take_the_tail() {
    for (name, target, encode, cmd) in all_commands() {
        assert!(
            !has(&cmd, "-ss"),
            "{name}/{encode:?}: -ss must never appear: {cmd:?}"
        );

        match target.trim_to {
            None => assert!(
                !has(&cmd, "-t"),
                "{name}/{encode:?}: -t with nothing to trim"
            ),
            Some(_) => {
                let t = index_of(&cmd, "-t").expect("a trim must be asked for");
                let i = index_of(&cmd, "-i").expect("there is always an input");
                assert!(
                    t > i,
                    "{name}/{encode:?}: -t before -i is an input seek: {cmd:?}"
                );
            }
        }
    }
}

/// The one place `-ss` is allowed to exist, and what it costs to use it.
///
/// The server's round never cuts an opening: it edits charts belonging to other
/// people and shifting their picture against the music is damage nobody asked
/// for. That is why [`plan::command`] has none and why the test above asserts it
/// never will.
///
/// This is the other case. The charter is the author, looking at their own
/// footage, deciding its first two seconds are a slate. The capability lives in
/// a separately named function so the two acts cannot be confused, and so the
/// invariant on the automatic path stays absolute rather than conditional.
#[test]
fn an_opening_cut_is_a_separate_act_with_its_own_rules() {
    let target = Target {
        size: Some((2560, 1440)),
        fps: Some(60),
        trim_to: Some(90.0),
        must_reencode: true,
        may_try_av1: false,
    };
    let cmd =
        plan::command_with_opening_cut("IN.mp4", "OUT.mp4", &target, plan::CRF, 2.5, Some(90.0));

    let ss = index_of(&cmd, "-ss").expect("this is the function that seeks");
    let i = index_of(&cmd, "-i").expect("there is always an input");
    assert!(
        ss > i,
        "-ss must sit AFTER -i: before it, ffmpeg jumps to the nearest keyframe \
         instead of the frame the charter picked. {cmd:?}"
    );
    assert_eq!(cmd[ss + 1], "2.500", "the cut is where it was asked for");

    // It can never be a stream copy. A copy starts at a keyframe -- seconds from
    // the handle -- so it would land somewhere nobody chose.
    assert!(has(&cmd, "-c:v") && has(&cmd, "libsvtav1"), "{cmd:?}");
    assert!(
        !has(&cmd, "copy"),
        "an opening cut cannot be a remux: {cmd:?}"
    );

    // Everything the ordinary recipe guarantees still holds.
    assert!(has(&cmd, "-an"), "{cmd:?}");
    let filters = index_of(&cmd, "-vf")
        .map(|i| cmd[i + 1].clone())
        .unwrap_or_default();
    assert!(filters.contains("scale=2560:1440"), "{filters}");
    assert!(filters.contains("fps=60"), "{filters}");
    assert_eq!(cmd.last().map(String::as_str), Some("OUT.mp4"));

    // With nothing to keep past the cut there is no `-t` at all, rather than a
    // `-t` of the whole remaining length: an argument that says nothing is an
    // argument that can be wrong.
    let open_ended =
        plan::command_with_opening_cut("IN.mp4", "OUT.mp4", &target, plan::CRF, 2.5, None);
    assert!(!has(&open_ended, "-t"), "{open_ended:?}");
}

/// Moving the opening is never something Press decides. `Trim::default()` leaves
/// it alone, and only a charter dragging the handle can change that.
#[test]
fn the_opening_is_never_moved_on_the_charters_behalf() {
    let untouched = plan::Trim::default();
    assert_eq!(untouched.cut_opening, 0.0);
    assert!(!untouched.moves_the_opening());
    assert!(untouched.is_untouched());
    assert_eq!(
        untouched.allowance(240.0),
        None,
        "no trim means no length limit"
    );
}

/// The audio track always goes. The game plays `music.ogg`; a background video's
/// own audio is decoded by nobody and shipped to everybody -- about 0.66 GB
/// across the catalogue when the gate first measured it.
#[test]
fn the_audio_track_is_dropped_on_every_path() {
    for (name, _, encode, cmd) in all_commands() {
        assert!(has(&cmd, "-an"), "{name}/{encode:?}: {cmd:?}");
    }
}

/// The whole value of the copy path is that the picture is not touched. If it
/// ever grew a filter or a quality setting it would be a re-encode wearing the
/// name of a remux, and the "AV1 must beat an exact copy" rule would be
/// comparing two lossy files.
#[test]
fn the_copy_path_never_touches_the_picture() {
    for scenario in &recipe().scenarios {
        let target = plan::plan(&facts_of(&scenario.facts), scenario.allowance);
        let cmd = plan::command("IN.mp4", "OUT.mp4", &target, Encode::Copy);
        let name = &scenario.name;

        assert!(has(&cmd, "-c") && has(&cmd, "copy"), "{name}: {cmd:?}");
        for forbidden in ["-vf", "-crf", "-c:v", "-preset", "-pix_fmt"] {
            assert!(
                !has(&cmd, forbidden),
                "{name}: copy path carries {forbidden}: {cmd:?}"
            );
        }
    }
}

/// A re-encode clamps exactly what the plan asked it to clamp -- no more, and
/// crucially no less. A missing `fps=` here is a file that still fails the gate
/// after all the CPU has been spent on it.
#[test]
fn a_reencode_clamps_what_the_plan_asked_for() {
    for scenario in &recipe().scenarios {
        let target = plan::plan(&facts_of(&scenario.facts), scenario.allowance);
        let cmd = plan::command("IN.mp4", "OUT.mp4", &target, Encode::Av1(plan::CRF));
        let name = &scenario.name;

        assert!(
            has(&cmd, "-c:v") && has(&cmd, "libsvtav1"),
            "{name}: {cmd:?}"
        );

        let filters = index_of(&cmd, "-vf")
            .map(|i| cmd[i + 1].clone())
            .unwrap_or_default();
        match target.size {
            None => assert!(
                !filters.contains("scale="),
                "{name}: scaling nothing: {filters}"
            ),
            Some((w, h)) => assert!(
                filters.contains(&format!("scale={w}:{h}")),
                "{name}: expected scale={w}:{h} in {filters:?}"
            ),
        }
        match target.fps {
            None => assert!(
                !filters.contains("fps="),
                "{name}: clamping nothing: {filters}"
            ),
            Some(fps) => assert!(
                filters.contains(&format!("fps={fps}")),
                "{name}: expected fps={fps} in {filters:?}"
            ),
        }
    }
}

/// The output path is the last argument, so nothing can be appended after it
/// and be read as an option applying to it.
#[test]
fn the_output_is_the_last_argument() {
    for (name, _, encode, cmd) in all_commands() {
        assert_eq!(
            cmd.last().map(String::as_str),
            Some("OUT.mp4"),
            "{name}/{encode:?}"
        );
    }
}

/// Progress reporting is spliced in by the runner, never baked into the recipe:
/// it changes what ffmpeg SAYS, not what it writes, and keeping it out is what
/// lets `command()` stay comparable with the recipe the server runs.
#[test]
fn progress_flags_are_not_part_of_the_recipe() {
    for (name, _, encode, cmd) in all_commands() {
        for flag in plan::PROGRESS_FLAGS {
            assert!(
                !has(&cmd, flag),
                "{name}/{encode:?}: {flag} leaked into the recipe"
            );
        }
    }
}

/// SVT-AV1 prints its banner and full configuration through its own logger,
/// which `-loglevel error` does not reach -- those lines come from the encoder
/// library, not from ffmpeg. The server's script sends stderr to /dev/null and
/// never sees them; Press keeps stderr in order to explain a real failure, so it
/// drops these itself or the one line that matters arrives buried under twenty
/// that do not.
#[test]
fn the_encoders_own_chatter_is_not_mistaken_for_a_failure() {
    for noise in [
        "Svt[info]: SVT [version]:\tSVT-AV1 Encoder Lib v4.1.0",
        "Svt[warn]: Non-RTC M10+ are meant for automation tooling usage.",
        "  Svt[info]: Level of Parallelism: 6",
    ] {
        assert!(plan::is_encoder_noise(noise), "should be dropped: {noise}");
    }
    for real in [
        "Error opening output file OUT.mp4.",
        "[out#0/mp4 @ 000001e3] Output file does not contain any stream",
        "",
    ] {
        assert!(!plan::is_encoder_noise(real), "should be kept: {real}");
    }
}
