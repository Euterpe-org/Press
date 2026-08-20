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
        Some(ffmpeg) => Some(ffmpeg.binary().to_path_buf()),
        None => {
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
    preview.play(0.0);
    await_frame(&preview);

    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline && !preview.ended() {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(preview.ended(), "a one-second source should have finished");
    assert!(
        !preview.playing(),
        "and should no longer report itself as playing"
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

/// A playing preview has to know where it is, at the rate the source runs at.
///
/// It did not. Frames come down a rawvideo pipe with no timestamps, so position
/// is counted forward from the seek — and the step was hard-coded to a thirtieth
/// of a second whatever the source was. `-re` paces the pipe at the source's
/// real rate, so on this 120 fps fixture the picture ran at speed while the
/// readout crawled at a quarter of it: press play and the video moves, the
/// playhead barely does.
///
/// Measured against the WALL CLOCK, because that is the claim: a second of
/// playing is a second of position. The band is wide enough for a loaded machine
/// and still nowhere near the four-fold error it is there to catch.
#[test]
fn a_playing_preview_advances_at_the_source_rate() {
    let Some(ffmpeg) = ffmpeg_path() else { return };

    let preview = Preview::open(&ffmpeg, &fixture("fps120.mp4"), 320, 180, 120.0);
    preview.play(0.0);
    // Only the newest frame is kept, so how far playback has already got by the
    // time the first one is observed is not fixed. The claim is about the RATE,
    // so both ends of the interval are measured rather than assumed.
    let first = await_frame(&preview);

    let began = Instant::now();
    std::thread::sleep(Duration::from_millis(350));
    let elapsed = began.elapsed().as_secs_f64();

    let Some(later) = preview.frame() else {
        panic!("a third of a second of playing produced no frame at all");
    };
    let advanced = later.pts - first.pts;
    assert!(
        advanced > elapsed * 0.5,
        "{elapsed:.2}s of playing advanced the position by only {advanced:.3}s — \
         the readout is counting at the wrong rate"
    );
    assert!(
        advanced < elapsed * 1.6,
        "{elapsed:.2}s of playing advanced the position by {advanced:.3}s — \
         faster than the source runs"
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
