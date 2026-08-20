//! Running ffmpeg, and hearing back from it.
//!
//! Press shells out rather than linking libavcodec, for one reason: the recipe
//! is the server's, and an argument list is the only form in which it can be
//! reproduced EXACTLY. A library binding would be a second implementation of
//! the recipe wearing the recipe's name, and the whole point is that a file
//! standardised here is the file the server's round would have produced.
//!
//! Two things make that harder than `Command::output()`:
//!
//! * **Progress.** An encode runs for tens of seconds and the window must not
//!   look hung, so `-progress pipe:1` is parsed off stdout as it arrives.
//! * **Noise.** SVT-AV1 prints its banner and entire configuration through its
//!   own logger, which `-loglevel error` does not reach -- those lines come from
//!   the encoder library, not from ffmpeg. The server's script sends stderr to
//!   /dev/null and never sees them. Press keeps stderr in order to explain a
//!   real failure, so it drops them itself; otherwise the one line that matters
//!   arrives buried under twenty that do not.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::plan;

/// A request to stop, shared between the window and the worker.
///
/// One token covers a whole run, so cancelling during the encode also prevents
/// the next rung of the CRF ladder from starting. Cloning shares the flag.
#[derive(Debug, Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn request(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn requested(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

/// How a run ended, short of failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ran {
    ToCompletion,
    /// Stopped on request. The output is a partial file and the caller deletes
    /// it -- a truncated MP4 left on disk is worse than none, because it looks
    /// like a result.
    Cancelled,
}

/// How far along an encode is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Progress {
    /// Position in the output, in seconds.
    pub out_time: f64,
    /// 0.0..=1.0 when the total length is known.
    pub fraction: Option<f64>,
    /// Bytes written so far, as ffmpeg last reported them.
    pub bytes: u64,
    /// Multiple of realtime, e.g. 3.5 for 3.5x. `None` before the first sample.
    pub speed: Option<f64>,
}

/// What a finished run leaves behind.
#[derive(Debug)]
pub struct Failure {
    pub status: Option<i32>,
    /// stderr with the encoder's own chatter removed. Empty is possible and is
    /// still a failure -- ffmpeg can exit non-zero having said nothing useful.
    pub stderr: String,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.status, self.stderr.trim()) {
            (_, complaint) if !complaint.is_empty() => write!(f, "{complaint}"),
            (Some(code), _) => write!(f, "ffmpeg exited with status {code}"),
            (None, _) => write!(f, "ffmpeg was terminated"),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// The binary could not be started at all.
    NotRunnable(std::io::Error),
    /// It ran and refused.
    Refused(Failure),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunnable(e) => write!(f, "ffmpeg could not be started: {e}"),
            Self::Refused(failure) => write!(f, "{failure}"),
        }
    }
}

/// The encoder Press runs.
#[derive(Debug, Clone)]
pub struct Ffmpeg {
    binary: PathBuf,
}

impl Ffmpeg {
    /// Use a specific binary. Release builds hand this the copy they extracted;
    /// nothing else should guess.
    pub fn at(binary: impl Into<PathBuf>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    /// Whatever is on PATH.
    ///
    /// For development only, and it is not a fallback: a shipped Press carries
    /// its own encoder and a machine's stray ffmpeg must never be able to change
    /// what it produces. Two builds of Press disagreeing about a chart because
    /// one of them found a different encoder is exactly the failure the recipe
    /// exists to prevent.
    pub fn on_path() -> Self {
        Self::at("ffmpeg")
    }

    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// Run one command to completion, reporting progress as it arrives.
    ///
    /// Blocking. The caller owns the thread -- the window runs on its own and
    /// hears about this through a channel.
    pub fn run(
        &self,
        args: &[String],
        total_seconds: Option<f64>,
        cancel: &Cancel,
        mut on_progress: impl FnMut(Progress),
    ) -> Result<Ran, Error> {
        // `args[0]` is the program NAME, not an argument. It is there because
        // the recipe is compared argument for argument against the list the
        // server hands to `subprocess.run`, where argv[0] is the program -- so
        // it is dropped here and the resolved binary takes its place. Passing it
        // through makes ffmpeg read "ffmpeg" as the output filename, which fails
        // with a muxer error that names neither the cause nor the recipe.
        let (program, args) = args.split_first().expect("a recipe is never empty");
        debug_assert_eq!(
            program, "ffmpeg",
            "the recipe's argv[0] is the program placeholder; got {program:?}"
        );

        // The recipe never carries these; they change what ffmpeg SAYS, not what
        // it writes.
        let mut full: Vec<String> = Vec::with_capacity(args.len() + plan::PROGRESS_FLAGS.len());
        full.extend(plan::PROGRESS_FLAGS.iter().map(|s| (*s).to_owned()));
        full.extend(args.iter().cloned());

        let mut child = Command::new(&self.binary)
            .args(&full)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(Error::NotRunnable)?;

        // stderr is drained on its own thread. A pipe nobody reads fills, and a
        // full pipe blocks the writer -- an encoder that dies of its own log is
        // a hang with no explanation, which is the worst shape a failure can
        // take here.
        let stderr = child.stderr.take().expect("stderr was piped");
        let collector = std::thread::spawn(move || {
            BufReader::new(stderr)
                .lines()
                .map_while(Result::ok)
                .filter(|line| !plan::is_encoder_noise(line))
                .collect::<Vec<_>>()
                .join("\n")
        });

        let stdout = child.stdout.take().expect("stdout was piped");
        let mut sample = ProgressSample::default();
        let mut stopped = false;
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            // Checked per line rather than per second: `-progress` emits a block
            // roughly twice a second, which is as responsive as a cancel button
            // needs to be, and it costs an atomic load.
            if cancel.requested() {
                let _ = child.kill();
                stopped = true;
                break;
            }
            if let Some(progress) = sample.absorb(&line, total_seconds) {
                on_progress(progress);
            }
        }

        let status = child.wait().map_err(Error::NotRunnable)?;
        let stderr = collector.join().unwrap_or_default();

        // A killed process exits non-zero and complains. That is not a failure
        // to report -- it is what being cancelled looks like from the outside.
        if stopped || cancel.requested() {
            return Ok(Ran::Cancelled);
        }
        if status.success() {
            Ok(Ran::ToCompletion)
        } else {
            Err(Error::Refused(Failure {
                status: status.code(),
                stderr,
            }))
        }
    }
}

/// One `-progress` block, accumulated until its terminating `progress=` line.
///
/// ffmpeg emits the keys one per line and closes each block with
/// `progress=continue` or `progress=end`, so a partially-read block would
/// otherwise be reported with fields from two different instants.
#[derive(Default)]
struct ProgressSample {
    out_time_us: Option<i64>,
    bytes: Option<u64>,
    speed: Option<f64>,
}

impl ProgressSample {
    fn absorb(&mut self, line: &str, total_seconds: Option<f64>) -> Option<Progress> {
        let (key, value) = line.split_once('=')?;
        match key.trim() {
            // Negative and "N/A" both appear before the first frame is written.
            "out_time_us" => self.out_time_us = value.trim().parse().ok().filter(|v| *v >= 0),
            "total_size" => self.bytes = value.trim().parse().ok(),
            "speed" => self.speed = value.trim().trim_end_matches('x').parse().ok(),
            "progress" => {
                let out_time = self.out_time_us.unwrap_or(0) as f64 / 1e6;
                let done = value.trim() == "end";
                let fraction = total_seconds.filter(|t| *t > 0.0).map(|t| {
                    if done {
                        1.0
                    } else {
                        (out_time / t).clamp(0.0, 1.0)
                    }
                });
                let sample = Progress {
                    out_time,
                    fraction,
                    bytes: self.bytes.unwrap_or(0),
                    speed: self.speed,
                };
                *self = Self::default();
                return Some(sample);
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fields arrive one per line and the block is closed by `progress=`.
    /// Reporting before that would mix two instants: `out_time_us` from this
    /// sample and `speed` from the last.
    #[test]
    fn a_sample_is_reported_only_when_its_block_closes() {
        let mut sample = ProgressSample::default();
        assert!(sample.absorb("frame=241", Some(10.0)).is_none());
        assert!(sample.absorb("total_size=4868800", Some(10.0)).is_none());
        assert!(sample.absorb("out_time_us=1999983", Some(10.0)).is_none());
        assert!(sample.absorb("speed=2.48x", Some(10.0)).is_none());

        let reported = sample
            .absorb("progress=continue", Some(10.0))
            .expect("closed");
        assert_eq!(reported.bytes, 4_868_800);
        assert_eq!(reported.speed, Some(2.48));
        assert!((reported.out_time - 1.999983).abs() < 1e-9);
        assert!((reported.fraction.unwrap() - 0.1999983).abs() < 1e-9);
    }

    /// `progress=end` means finished, whatever the last `out_time_us` said. A
    /// trim stops the output early on purpose, so the position would otherwise
    /// leave the bar short of the end for exactly the files that were trimmed.
    #[test]
    fn the_end_of_the_run_is_a_full_bar_even_after_a_trim() {
        let mut sample = ProgressSample::default();
        sample.absorb("out_time_us=90000000", Some(238.6));
        let reported = sample.absorb("progress=end", Some(238.6)).expect("closed");
        assert_eq!(reported.fraction, Some(1.0));
    }

    /// Before the first frame ffmpeg reports a negative `out_time_us`, and with
    /// no length to divide by there is no fraction to report at all.
    #[test]
    fn a_position_before_the_first_frame_is_not_a_position() {
        let mut sample = ProgressSample::default();
        sample.absorb("out_time_us=-38000", Some(10.0));
        let reported = sample
            .absorb("progress=continue", Some(10.0))
            .expect("closed");
        assert_eq!(reported.out_time, 0.0);
        assert_eq!(reported.fraction, Some(0.0));

        let mut unknown = ProgressSample::default();
        unknown.absorb("out_time_us=5000000", None);
        assert_eq!(
            unknown.absorb("progress=continue", None).unwrap().fraction,
            None
        );
    }

    /// A run that says nothing useful still has to produce a sentence: "it
    /// failed" with no reason is the shape that sends someone hunting.
    #[test]
    fn a_silent_failure_still_names_itself() {
        let quiet = Failure {
            status: Some(1),
            stderr: String::new(),
        };
        assert_eq!(quiet.to_string(), "ffmpeg exited with status 1");

        let killed = Failure {
            status: None,
            stderr: "  \n ".to_owned(),
        };
        assert_eq!(killed.to_string(), "ffmpeg was terminated");

        let spoke = Failure {
            status: Some(1),
            stderr: "Invalid argument".to_owned(),
        };
        assert_eq!(spoke.to_string(), "Invalid argument");
    }
}
