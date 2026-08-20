//! The viewfinder actually decodes.
//!
//! Everything else about Press is judged from an MP4 header; this is the one
//! part that has to get real pixels out of a real decoder, so it is the one
//! part a unit test cannot stand in for. The interesting case is a codec no
//! system video player would open -- which is precisely the file Press exists
//! to fix.
//!
//! Needs ffmpeg on PATH; skips rather than fails without one.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use euterpe_press::preview::{MAX_EDGE, Preview, preview_size};

fn ffmpeg_path() -> Option<PathBuf> {
    let ok = std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        Some(PathBuf::from("ffmpeg"))
    } else {
        eprintln!("skipping: no ffmpeg on PATH");
        None
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// Waits for the decoder to hand over a frame, or gives up.
fn await_frame(preview: &Preview) -> euterpe_press::preview::Frame {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if let Some(frame) = preview.frame() {
            return frame;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("no frame arrived within the deadline");
}

/// A frame that is entirely one colour is what a torn read, a wrong pixel
/// format or a seek past the end all look like. `testsrc2` is never flat.
fn is_flat(rgba: &[u8]) -> bool {
    let first = &rgba[..4];
    rgba.chunks_exact(4).all(|pixel| pixel == first)
}

#[test]
fn a_frame_comes_back_at_the_size_it_was_asked_for() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180);
    assert_eq!(preview.size(), (320, 180), "already inside the cap, so unscaled");

    preview.seek(0.5);
    let frame = await_frame(&preview);

    assert_eq!((frame.width, frame.height), (320, 180));
    assert_eq!(
        frame.rgba.len(),
        320 * 180 * 4,
        "a short read is a torn frame, and a torn frame looks like a broken source"
    );
    assert!(!is_flat(&frame.rgba), "the fixture has content; a flat frame means it was not decoded");
}

/// The whole reason the viewfinder is built on the encoder rather than on a
/// system video element: VP9 in MP4 is a file Press has to be able to SHOW in
/// order to help fix, and it is exactly what a browser or Media Foundation
/// declines to open.
#[test]
fn a_codec_no_player_would_open_still_previews() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    for (file, w, h) in [("vp9.mp4", 320, 180), ("mpeg4.mp4", 320, 180)] {
        let preview = Preview::open(&ffmpeg, &fixture(file), w, h);
        preview.seek(0.2);
        let frame = await_frame(&preview);
        assert_eq!((frame.width, frame.height), (w, h), "{file}");
        assert!(!is_flat(&frame.rgba), "{file}: decoded to a flat frame");
    }
}

/// A large source is scaled down before it ever reaches the pipe: full size
/// would be eight times the bandwidth for a picture the window never shows
/// larger than the cap.
#[test]
fn a_large_source_is_scaled_before_it_reaches_the_pipe() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let (w, h) = preview_size(2560, 1440, MAX_EDGE);
    assert_eq!((w, h), (640, 360));

    let preview = Preview::open(&ffmpeg, &fixture("pixels_at_limit.mp4"), 2560, 1440);
    assert_eq!(preview.size(), (640, 360));

    preview.seek(0.3);
    let frame = await_frame(&preview);
    assert_eq!((frame.width, frame.height), (640, 360));
    assert_eq!(frame.rgba.len(), 640 * 360 * 4);
}

/// Playing runs to the end of a one-second source and then says so, rather than
/// leaving the window on a spinner that never resolves.
#[test]
fn playing_to_the_end_reports_that_it_ended() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180);
    preview.play(0.0);
    await_frame(&preview);

    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !preview.ended() {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(preview.ended(), "a one-second source should have finished");
    assert!(!preview.playing(), "and should no longer report itself as playing");
}

/// Dropping the preview has to stop the decoder. A viewfinder that leaves an
/// ffmpeg running per file the charter opened would quietly eat the machine.
#[test]
fn dropping_the_preview_stops_its_decoder() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180);
    preview.play(0.0);
    await_frame(&preview);
    // Drop joins the worker, which drops the Decoder, which kills the child.
    // If either link were missing this would hang rather than return.
    drop(preview);
}
