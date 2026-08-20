//! The upload gate: does the server accept this file?
//!
//! Press exists to answer that question before an 80 MB upload, so the answer
//! has to be the server's own. The rule lives in
//! `Euterpe.Web/backend/app/domain/video_probe.py::check` and is mirrored here,
//! in the same order, with the same codes and the same params -- so a file
//! refused by Press and a file refused by the server produce one sentence, in
//! whichever of the fourteen languages the user reads.
//!
//! The order is not cosmetic. A structural problem is named BEFORE the audio
//! track, because stripping the audio cannot fix a wrong resolution, a wrong
//! frame rate or a codec nobody can decode -- all three need a fresh export.
//! Reporting the audio first sends the charter round the loop twice: they do
//! what the message said, re-upload, and are refused again for the thing that
//! actually mattered.

pub mod limits;
pub mod probe;

pub use probe::{Facts, probe};

/// Why the server would refuse this file, or `Ok` if it would not.
///
/// A closed enum rather than a code plus a string map: every variant carries
/// exactly the values its sentence interpolates, so a message cannot be
/// rendered with a parameter the code never produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    /// Megabytes, both already rounded UP. A file one byte over the limit rounds
    /// DOWN to it, and "your video is 80 MB, the limit is 80 MB" reads as a bug
    /// in the gate rather than as a fact about the file.
    TooLarge { size_mb: u64, limit_mb: u64 },
    /// Not an MP4, no video track, truncated, or no usable timing. One outcome
    /// on purpose: they all mean "re-export as a standard MP4".
    Invalid,
    FormatUnsupported { codec: String },
    ResolutionTooHigh { width: u32, height: u32, max_width: u32, max_height: u32 },
    FpsTooHigh { fps: i64, limit: u32 },
    HasAudio,
}

impl Verdict {
    /// The `errors:codes.*` key the web and the browser use for this same
    /// verdict. Press renders its own translations, but from the same key, so
    /// the three surfaces cannot describe one file three ways.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::TooLarge { .. } => "VIDEO_TOO_LARGE",
            Self::Invalid => "INVALID_VIDEO",
            Self::FormatUnsupported { .. } => "VIDEO_FORMAT_UNSUPPORTED",
            Self::ResolutionTooHigh { .. } => "VIDEO_RESOLUTION_TOO_HIGH",
            Self::FpsTooHigh { .. } => "VIDEO_FPS_TOO_HIGH",
            Self::HasAudio => "VIDEO_HAS_AUDIO",
        }
    }

    pub const fn passes(&self) -> bool {
        matches!(self, Self::Ok)
    }
}

const MB: u64 = 1024 * 1024;

const fn ceil_mb(bytes: u64) -> u64 {
    bytes.div_ceil(MB)
}

/// The gate. Returns the FIRST violation, in the server's order.
///
/// Size is checked before the header is even consulted: it is the one limit that
/// costs nothing to measure and the one the other three exist to protect.
pub fn check(size: u64, facts: Option<&Facts>) -> Verdict {
    if size > limits::MAX_BYTES {
        return Verdict::TooLarge {
            size_mb: ceil_mb(size),
            limit_mb: ceil_mb(limits::MAX_BYTES),
        };
    }
    let Some(facts) = facts else {
        return Verdict::Invalid;
    };
    if !limits::codec_allowed(facts.codec) {
        return Verdict::FormatUnsupported {
            codec: if facts.raw_fourcc.is_empty() {
                "unknown".to_owned()
            } else {
                facts.raw_fourcc.clone()
            },
        };
    }
    if u64::from(facts.width) * u64::from(facts.height) > limits::MAX_PIXELS {
        return Verdict::ResolutionTooHigh {
            width: facts.width,
            height: facts.height,
            max_width: limits::MAX_WIDTH,
            max_height: limits::MAX_HEIGHT,
        };
    }
    if probe::over_fps_limit(facts.fps) {
        return Verdict::FpsTooHigh {
            fps: probe::reported_fps(facts.fps),
            limit: limits::MAX_FPS,
        };
    }
    if facts.has_audio {
        // Last, because it is the cheapest violation to fix and by far the most
        // common -- 85% of the live library carries one. Everything above it
        // needs a fresh export; this one is a remux.
        return Verdict::HasAudio;
    }
    Verdict::Ok
}

/// Read a file and judge it. The only function most callers need.
pub fn judge(bytes: &[u8]) -> (Option<Facts>, Verdict) {
    let facts = probe(bytes);
    let verdict = check(bytes.len() as u64, facts.as_ref());
    (facts, verdict)
}
