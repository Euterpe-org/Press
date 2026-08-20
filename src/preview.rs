//! Showing the charter their own video, so the trim can be aimed.
//!
//! There is no `<video>` to lean on here and there would not be one in a
//! browser-based build either: the files Press exists to fix are the ones a
//! player cannot open. A VP9, mp4v or HEVC source is exactly the case that
//! matters, and it is exactly the case a system video element refuses. So
//! frames come from the same encoder that does the work: ffmpeg decodes to raw
//! RGBA, scaled down to a preview size, straight down a pipe.
//!
//! Two modes, and they are the same pipeline:
//!
//! * **Scrubbing** wants one frame at a position, now. `-frames:v 1` and done.
//! * **Playing** wants them at the rate they were shot. `-re` makes ffmpeg pace
//!   its own input, which is what keeps a preview from running at decode speed
//!   -- reading as fast as the pipe delivers and drawing the newest frame would
//!   otherwise play a three-minute video in four seconds.
//!
//! Seeking is a restart. It costs about fifty milliseconds and it means there is
//! exactly one piece of state to reason about: either a process is running or
//! the picture is frozen.
//!
//! **The preview is a viewfinder, never a verdict.** It is scaled, it seeks to
//! keyframes, and nothing it shows is used to judge a file -- that is the gate's
//! job, on the bytes, through the header.

use std::io::Read;
use std::path::Path;
use std::process::{Child, Stdio};

use crate::ffmpeg::quietly;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};

/// The longest edge a preview frame is allowed to have.
///
/// 640 keeps a 16:9 frame at 640x360, which is 900 KB of RGBA -- 27 MB/s at
/// 30 fps down a pipe, comfortable, and more resolution than a viewfinder for
/// aiming a trim needs. Full size would be eight times that for no benefit: the
/// window never shows it larger than this.
pub const MAX_EDGE: u32 = 640;

/// Two seek requests this close together are the same moment.
///
/// A held scrubber reports its position every frame; without this, standing
/// still would decode the picture already on screen, over and over. A fortieth
/// of a second is finer than the frame interval of any source up to 40 fps, and
/// far finer than a viewfinder needs.
const SAME_MOMENT: f64 = 0.025;

/// One decoded frame, ready to become a texture.
#[derive(Clone)]
pub struct Frame {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, top-down.
    pub rgba: Arc<Vec<u8>>,
    /// Where this frame sits in the source, in seconds.
    pub pts: f64,
}

enum Request {
    /// Show one frame here and stop.
    Seek(f64),
    /// Run from here at the source's own rate.
    Play(f64),
    Pause,
    Shutdown,
}

struct Shared {
    /// Only the newest frame is kept. A preview that queues frames falls
    /// steadily further behind the sound in the charter's head; one that drops
    /// them stays honest about where it is.
    latest: Mutex<Option<Frame>>,
    playing: AtomicBool,
    /// Set when the decoder reached the end of the source, so the window can
    /// show the play button again rather than a spinner that never resolves.
    ended: AtomicBool,
    /// How many decoder processes have actually been started.
    ///
    /// Observable because the difference between "one seek, one process" and
    /// "one seek per frame, sixty processes a second" is otherwise invisible
    /// from outside -- and that difference was a strobe of console windows that
    /// looked like malware.
    decodes: AtomicU64,
}

/// A running preview. Dropping it stops the decoder.
pub struct Preview {
    shared: Arc<Shared>,
    commands: Sender<Request>,
    size: (u32, u32),
    worker: Option<std::thread::JoinHandle<()>>,
}

impl Preview {
    /// Start a decoder for `source`, sized from the frame it actually carries
    /// and clocked at the rate it was shot.
    ///
    /// `fps` is not decoration. Frames come down a rawvideo pipe with no
    /// timestamps on them, so a playing preview knows where it is only by
    /// counting forward from where the seek landed -- and counting at the wrong
    /// rate is a playhead that drifts away from the picture it is supposed to be
    /// pointing at. At a hard-coded 30 a 60 fps source ran the readout at half
    /// speed and a 24 fps one at a quarter again too fast.
    pub fn open(ffmpeg: &Path, source: &Path, width: u32, height: u32, fps: f64) -> Self {
        let size = preview_size(width, height, MAX_EDGE);
        // A source that reports nothing usable still has to play at SOME rate.
        let step = if fps.is_finite() && fps > 1.0 {
            1.0 / fps
        } else {
            1.0 / 30.0
        };
        let shared = Arc::new(Shared {
            latest: Mutex::new(None),
            playing: AtomicBool::new(false),
            ended: AtomicBool::new(false),
            decodes: AtomicU64::new(0),
        });
        let (commands, inbox) = channel();

        let worker = std::thread::spawn({
            let shared = Arc::clone(&shared);
            let ffmpeg = ffmpeg.to_path_buf();
            let source = source.to_path_buf();
            move || decode_loop(&ffmpeg, &source, size, step, &shared, &inbox)
        });

        Self {
            shared,
            commands,
            size,
            worker: Some(worker),
        }
    }

    /// The frame size every [`Frame`] will carry.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn seek(&self, seconds: f64) {
        let _ = self.commands.send(Request::Seek(seconds.max(0.0)));
    }

    pub fn play(&self, from: f64) {
        let _ = self.commands.send(Request::Play(from.max(0.0)));
    }

    pub fn pause(&self) {
        let _ = self.commands.send(Request::Pause);
    }

    pub fn playing(&self) -> bool {
        self.shared.playing.load(Ordering::Relaxed)
    }

    /// True once the decoder has run off the end of the source.
    pub fn ended(&self) -> bool {
        self.shared.ended.load(Ordering::Relaxed)
    }

    /// How many decoder processes this preview has started.
    pub fn decodes(&self) -> u64 {
        self.shared.decodes.load(Ordering::Relaxed)
    }

    /// The newest frame, if one has arrived since the decoder started.
    pub fn frame(&self) -> Option<Frame> {
        self.shared.latest.lock().ok().and_then(|f| f.clone())
    }
}

impl Drop for Preview {
    fn drop(&mut self) {
        let _ = self.commands.send(Request::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// Fit inside `max_edge`, on even dimensions.
///
/// Even because yuv420p subsamples by two and an odd request makes ffmpeg
/// round on its own -- at which point the frames arriving down the pipe are not
/// the size the reader is slicing them into, and every frame after the first is
/// torn.
pub fn preview_size(width: u32, height: u32, max_edge: u32) -> (u32, u32) {
    let longest = width.max(height).max(1);
    if longest <= max_edge {
        return (even(width), even(height));
    }
    let scale = f64::from(max_edge) / f64::from(longest);
    (
        even((f64::from(width) * scale) as u32),
        even((f64::from(height) * scale) as u32),
    )
}

fn even(value: u32) -> u32 {
    (value / 2 * 2).max(2)
}

fn decode_loop(
    ffmpeg: &Path,
    source: &Path,
    size: (u32, u32),
    step: f64,
    shared: &Arc<Shared>,
    inbox: &Receiver<Request>,
) {
    let mut running: Option<Decoder> = None;
    let mut showing: Option<f64> = None;

    loop {
        // COALESCE. A drag on the scrubber produces a request per frame -- sixty
        // a second -- and each one starts a decoder. Acting on every request
        // means sixty processes a second, each killed by the next before it can
        // finish, which is a scrubber that shows nothing while pinning a core.
        //
        // Only the newest request matters: the charter is looking at where the
        // pointer is NOW. So the queue is drained to its last entry and the rest
        // are dropped unexecuted -- work nobody was ever going to see.
        let mut latest: Option<Request> = None;
        loop {
            match inbox.try_recv() {
                Ok(Request::Shutdown) => return,
                Err(TryRecvError::Disconnected) => return,
                Ok(request) => latest = Some(request),
                Err(TryRecvError::Empty) => break,
            }
        }

        match latest {
            // Kept, and never taken. The drain above returns the moment it sees
            // a shutdown, precisely so it cannot be coalesced away by a request
            // queued behind it -- so this arm exists for the compiler's benefit
            // rather than the thread's. Deleting the early return to "remove the
            // duplicate" would make shutdown depend on being the last message in
            // the queue, which is true today and is not a property anything
            // enforces.
            Some(Request::Shutdown) => return,
            Some(Request::Pause) => {
                running = None;
                shared.playing.store(false, Ordering::Relaxed);
                continue;
            }
            Some(Request::Seek(at)) => {
                shared.playing.store(false, Ordering::Relaxed);
                shared.ended.store(false, Ordering::Relaxed);
                running = None;
                // Already showing this moment. A scrubber held still still
                // reports its position every frame, and decoding the same frame
                // again would cost a process to produce the picture already on
                // screen.
                if showing.is_some_and(|shown| (shown - at).abs() < SAME_MOMENT) {
                    continue;
                }
                shared.decodes.fetch_add(1, Ordering::Relaxed);
                if let Some(frame) = still(ffmpeg, source, size, step, at) {
                    *shared.latest.lock().expect("preview frame") = Some(frame);
                    showing = Some(at);
                }
                continue;
            }
            Some(Request::Play(from)) => {
                shared.ended.store(false, Ordering::Relaxed);
                shared.decodes.fetch_add(1, Ordering::Relaxed);
                running = Decoder::start(ffmpeg, source, size, step, from, true);
                shared.playing.store(running.is_some(), Ordering::Relaxed);
                showing = None;
                continue;
            }
            None => {}
        }

        let Some(decoder) = running.as_mut() else {
            // Nothing to do until something is asked for. Sleeping rather than
            // spinning: a viewfinder must not cost a core while it sits still.
            std::thread::sleep(std::time::Duration::from_millis(15));
            continue;
        };

        match decoder.next_frame() {
            Some(frame) => *shared.latest.lock().expect("preview frame") = Some(frame),
            None => {
                running = None;
                shared.playing.store(false, Ordering::Relaxed);
                shared.ended.store(true, Ordering::Relaxed);
            }
        }
    }
}

/// One frame at a position, for the scrubber.
fn still(ffmpeg: &Path, source: &Path, size: (u32, u32), step: f64, at: f64) -> Option<Frame> {
    let mut decoder = Decoder::start(ffmpeg, source, size, step, at, false)?;
    decoder.next_frame()
}

struct Decoder {
    child: Child,
    size: (u32, u32),
    frame_bytes: usize,
    /// Frames carry no timestamp down a rawvideo pipe, so the position is
    /// counted forward from where the seek landed. Good enough for a viewfinder
    /// and never used to judge anything.
    next_pts: f64,
    frame_step: f64,
}

impl Decoder {
    fn start(
        ffmpeg: &Path,
        source: &Path,
        size: (u32, u32),
        step: f64,
        from: f64,
        realtime: bool,
    ) -> Option<Self> {
        let (w, h) = size;
        let mut command = quietly(ffmpeg);
        command.args(["-v", "error", "-nostdin"]);
        if realtime {
            // Makes ffmpeg read its input at the source's own frame rate. Without
            // it the pipe delivers as fast as the CPU decodes and a three-minute
            // video plays in four seconds.
            command.arg("-re");
        }
        if from > 0.0 {
            // Before `-i`: seeks by keyframe, which is fast and is what a
            // viewfinder wants. The trim itself never uses this -- see
            // `plan::command`, which has no `-ss` at all.
            command.args(["-ss", &format!("{from:.3}")]);
        }
        command
            .arg("-i")
            .arg(source)
            .args([
                "-an",
                "-sn",
                "-vf",
                &format!("scale={w}:{h}:flags=bilinear"),
            ])
            .args(["-f", "rawvideo", "-pix_fmt", "rgba"]);
        if !realtime {
            command.args(["-frames:v", "1"]);
        }
        let child = command
            .arg("-")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        Some(Self {
            child,
            size,
            frame_bytes: (w as usize) * (h as usize) * 4,
            next_pts: from,
            frame_step: step,
        })
    }

    /// Reads exactly one frame, or `None` at the end of the stream.
    ///
    /// Exactly: a short read is a torn frame, and a torn frame drawn once looks
    /// like a decoding bug in the source rather than a truncated pipe.
    fn next_frame(&mut self) -> Option<Frame> {
        let stdout = self.child.stdout.as_mut()?;
        let mut rgba = vec![0u8; self.frame_bytes];
        if stdout.read_exact(&mut rgba).is_err() {
            return None;
        }
        let pts = self.next_pts;
        self.next_pts += self.frame_step;
        Some(Frame {
            width: self.size.0,
            height: self.size.1,
            rgba: Arc::new(rgba),
            pts,
        })
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        // The child is reading a file and writing a pipe nobody will drain
        // again. Killing it is the only thing that ends it.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The frame Press hands the window while a source has not been chosen yet, and
/// the one it falls back to if a decode produces nothing at all.
pub fn blank(size: (u32, u32)) -> Frame {
    Frame {
        width: size.0,
        height: size.1,
        rgba: Arc::new(vec![0; (size.0 as usize) * (size.1 as usize) * 4]),
        pts: 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Odd dimensions make ffmpeg round on its own, and then the frames coming
    /// down the pipe are not the size the reader slices them into -- every frame
    /// after the first is torn.
    #[test]
    fn preview_frames_are_always_even() {
        for (w, h) in [
            (1920, 1080),
            (1080, 1920),
            (1281, 721),
            (3840, 2160),
            (7, 3),
        ] {
            let (pw, ph) = preview_size(w, h, MAX_EDGE);
            assert_eq!(pw % 2, 0, "{w}x{h} -> {pw}x{ph}");
            assert_eq!(ph % 2, 0, "{w}x{h} -> {pw}x{ph}");
            assert!(pw >= 2 && ph >= 2, "{w}x{h} -> {pw}x{ph}");
        }
    }

    /// Fitting is by the LONGEST edge, so a portrait source is bounded by its
    /// height the way a landscape one is bounded by its width. Three charts in
    /// the live library are 1080x1920.
    #[test]
    fn the_longest_edge_is_what_is_bounded() {
        assert_eq!(preview_size(1920, 1080, 640), (640, 360));
        assert_eq!(preview_size(1080, 1920, 640), (360, 640));
        assert_eq!(preview_size(2560, 1440, 640), (640, 360));
    }

    /// A source already smaller than the cap is shown at its own size rather
    /// than blown up: upscaling a viewfinder adds nothing and costs bandwidth.
    #[test]
    fn a_small_source_is_not_enlarged() {
        assert_eq!(preview_size(320, 180, 640), (320, 180));
        assert_eq!(preview_size(640, 360, 640), (640, 360));
    }
}
