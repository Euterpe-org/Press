//! The viewfinder actually decodes.
//!
//! Everything else about Press is judged from an MP4 header; this is the one
//! part that has to get real pixels out of a real decoder, so it is the one
//! part a unit test cannot stand in for. The interesting case is a codec no
//! system video player would open -- which is precisely the file Press exists
//! to fix.
//!
//! Runs against the encoder Press would ship; skips rather than fails when there
//! is none to run.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use euterpe_press::preview::{MAX_EDGE, Preview, preview_size};

fn ffmpeg_path() -> Option<PathBuf> {
    // The encoder Press would SHIP. A viewfinder that only works against the
    // developer's fuller ffmpeg is a viewfinder that fails on the product.
    match euterpe_press::encoder::resolve() {
        Ok(ffmpeg) => Some(ffmpeg.binary().to_path_buf()),
        Err(_) => {
            eprintln!("skipping: no encoder embedded and none on PATH");
            None
        }
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
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

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180, 30.0);
    assert_eq!(
        preview.size(),
        (320, 180),
        "already inside the cap, so unscaled"
    );

    preview.seek(0.5);
    let frame = await_frame(&preview);

    assert_eq!((frame.width, frame.height), (320, 180));
    assert_eq!(
        frame.rgba.len(),
        320 * 180 * 4,
        "a short read is a torn frame, and a torn frame looks like a broken source"
    );
    assert!(
        !is_flat(&frame.rgba),
        "the fixture has content; a flat frame means it was not decoded"
    );
}

/// The whole reason the viewfinder is built on the encoder rather than on a
/// system video element: VP9 in MP4 is a file Press has to be able to SHOW in
/// order to help fix, and it is exactly what a browser or Media Foundation
/// declines to open.
#[test]
fn a_codec_no_player_would_open_still_previews() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    for (file, w, h) in [("vp9.mp4", 320, 180), ("mpeg4.mp4", 320, 180)] {
        let preview = Preview::open(&ffmpeg, &fixture(file), w, h, 30.0);
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

    let preview = Preview::open(&ffmpeg, &fixture("pixels_at_limit.mp4"), 2560, 1440, 30.0);
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

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180, 30.0);
    let began = Instant::now();
    preview.play(0.0);
    await_frame(&preview);

    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !preview.ended() {
        std::thread::sleep(Duration::from_millis(20));
    }
    let took = began.elapsed().as_secs_f64();
    // READ AFTER IT STOPS, not while it runs. The decoder keeps its final frame
    // in place once it has finished, so this is the position it actually
    // reached. Sampling during playback instead is a race with no winner: the
    // whole second drains in about half of one, so a poll that lands late
    // reports wherever it last happened to look.
    let last = preview
        .frame()
        .expect("the final frame stays available")
        .pts;

    assert!(preview.ended(), "a one-second source should have finished");
    assert!(
        !preview.playing(),
        "and should no longer report itself as playing"
    );

    // WHERE IT ENDED UP, which is what "the position is counted at the source's
    // rate" actually means. Frames come down a rawvideo pipe carrying no
    // timestamps, so the readout is accumulated one frame step at a time. Get
    // that step wrong and a one-second source finishes reading half a second,
    // or two.
    //
    // Asserted on the VALUE, not on how long the playing took. The rate itself
    // is not measurable on a one-second fixture: `-re` paces ffmpeg's reading,
    // but only once its buffer is full, so sampled every 50 ms this source
    // reports nothing for ~150 ms and then arrives already a third of the way
    // in, and the whole clip drains in ~600 ms. A test that timed a window
    // inside that failed two runs in five, and moving the window only changed
    // which end it failed at.
    assert!(
        (0.8..=1.05).contains(&last),
        "a one-second source finished with the position at {last:.3}s — \
         the readout is counting at the wrong rate"
    );

    // A floor on the wall clock: far under the ~600 ms this really takes, far
    // over the ~50 ms it would take with the pacing removed. Not a measurement
    // of the rate, just proof that something is pacing at all.
    assert!(
        took > 0.2,
        "the whole source played in {took:.3}s, which is not playing, \
         it is draining a pipe"
    );
}

/// A drag on the scrubber sends a seek per frame. Acting on every one of them
/// starts sixty decoder processes a second, each killed by the next before it
/// can produce anything — a scrubber that shows nothing while pinning a core,
/// and, because a GUI process gives every console child a window, a strobe of
/// black rectangles that looks exactly like malware.
///
/// That is not a hypothetical: it is what shipped, and what a screenshot caught.
/// Only the newest request matters, so the queue is drained and the rest are
/// dropped unexecuted.
#[test]
fn a_burst_of_seeks_does_not_start_a_process_for_each_one() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180, 30.0);

    // A second of dragging, at sixty frames a second, across the clip. Stops
    // short of the very end: a one-second source has its last frame at 0.967,
    // and asking for a picture past that is asking for one that is not there.
    for step in 0..60 {
        preview.seek(f64::from(step) / 60.0 * 0.9);
    }
    await_frame(&preview);
    std::thread::sleep(Duration::from_millis(400));

    let decodes = preview.decodes();
    assert!(
        decodes < 10,
        "sixty seeks started {decodes} decoders; they are meant to coalesce"
    );
}

/// A scrubber held still reports its position every frame. Decoding the picture
/// already on screen, repeatedly, costs a process each time and changes nothing.
#[test]
fn holding_the_scrubber_still_decodes_once() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180, 30.0);
    preview.seek(0.5);
    await_frame(&preview);
    std::thread::sleep(Duration::from_millis(300));
    let after_first = preview.decodes();

    // Sixty frames of a pointer that has not moved.
    for _ in 0..60 {
        preview.seek(0.5);
        std::thread::sleep(Duration::from_millis(2));
    }
    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(
        preview.decodes(),
        after_first,
        "standing still decoded again; the same moment is the same picture"
    );
}

/// Dropping the preview has to stop the decoder. A viewfinder that leaves an
/// ffmpeg running per file the charter opened would quietly eat the machine.
#[test]
fn dropping_the_preview_stops_its_decoder() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let preview = Preview::open(&ffmpeg, &fixture("plain_h264.mp4"), 320, 180, 30.0);
    preview.play(0.0);
    await_frame(&preview);
    // Drop joins the worker, which drops the Decoder, which kills the child.
    // If either link were missing this would hang rather than return.
    drop(preview);
}
