//! The crop reaches ffmpeg and comes back as a real, SMALLER picture.
//!
//! Not a unit test of the geometry -- `crop`'s own tests do that -- but of the
//! chain: choice, plan, filter string, encoder, and the dimensions the file on
//! disk actually reports afterwards. Every link in that has been wrong at least
//! once in this project, and none of them fails loudly on its own.

use euterpe_press::crop::{Shape, window};
use euterpe_press::ffmpeg::{Cancel, Watching};
use euterpe_press::gate;
use euterpe_press::plan::{self, Encode};

#[test]
fn a_crop_survives_the_whole_chain() {
    let Some(ffmpeg) = euterpe_press::encoder::resolve() else {
        eprintln!("skipping: no encoder");
        return;
    };
    // Portrait: 1080x1920 is nothing like either screen shape, so both crops
    // are real and large.
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/portrait.mp4");
    let bytes = std::fs::read(&source).expect("fixture");
    let facts = gate::probe(&bytes).expect("readable");
    let from = (facts.width, facts.height);

    for shape in Shape::ALL {
        let cut = window(from, shape, 0.5).expect("portrait is neither shape");
        let target = plan::plan(&facts, None, Some(cut));
        let out = std::env::temp_dir().join("euterpe-crop-probe.mp4");
        let _ = std::fs::remove_file(&out);

        let cmd = plan::command(
            &source.to_string_lossy(),
            &out.to_string_lossy(),
            &target,
            Encode::Av1(45),
        );
        let chain = cmd.join(" ");
        assert!(
            chain.contains(&format!("crop={}:{}:", cut.width, cut.height)),
            "the crop never reached the argv: {chain}"
        );
        // Crop BEFORE scale, or the scale factor is computed for a picture that
        // no longer exists by the time it runs.
        if let (Some(c), Some(s)) = (chain.find("crop="), chain.find("scale=")) {
            assert!(c < s, "crop must come before scale: {chain}");
        }

        ffmpeg
            .run(&cmd, Some(facts.duration), &Cancel::new(), |_| {
                Watching::Continue
            })
            .expect("the encode should run");

        let made = std::fs::read(&out).expect("an output");
        let after = gate::probe(&made).expect("readable output");
        let (want_w, want_h) = target.frame_size(from);
        assert_eq!(
            (after.width, after.height),
            (want_w, want_h),
            "{shape:?}: the file on disk is not the shape the plan promised"
        );

        // And it is genuinely SMALLER, not the same canvas with black welded
        // into the edges.
        assert!(
            u64::from(after.width) * u64::from(after.height)
                < u64::from(from.0) * u64::from(from.1),
            "{shape:?}: a crop that did not shrink the picture is not a crop"
        );

        let (rw, rh) = shape.ratio();
        let got = f64::from(after.width) / f64::from(after.height);
        let wanted = f64::from(rw) / f64::from(rh);
        assert!(
            (got - wanted).abs() / wanted < 0.02,
            "{shape:?}: asked for {rw}:{rh}, got {}x{} = {got:.3}",
            after.width,
            after.height
        );
        println!("{shape:?}: {from:?} -> {}x{}", after.width, after.height);
        let _ = std::fs::remove_file(&out);
    }
}
