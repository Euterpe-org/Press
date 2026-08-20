//! Temporary command line, until the window exists.
//!
//! Kept afterwards regardless: the gate answering from a terminal is how a batch
//! of files gets checked without seventeen drag-and-drops, and how CI can prove
//! the shipped binary agrees with the server rather than only the test harness
//! doing so.

use std::path::PathBuf;
use std::process::ExitCode;

use euterpe_press::gate::{self, Verdict};

fn main() -> ExitCode {
    let files: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if files.is_empty() {
        eprintln!("usage: euterpe-press <video.mp4> [...]");
        return ExitCode::from(2);
    }

    let mut refused = false;
    for path in &files {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                println!("{:<40} could not be read: {error}", name(path));
                refused = true;
                continue;
            }
        };

        let (facts, verdict) = gate::judge(&bytes);
        let shape = match &facts {
            None => "unreadable".to_owned(),
            Some(f) => format!(
                "{} {}x{} {:.2}fps {:.1}s{}",
                f.codec.unwrap_or(if f.raw_fourcc.is_empty() { "?" } else { &f.raw_fourcc }),
                f.width,
                f.height,
                f.fps,
                f.duration,
                if f.has_audio { " +audio" } else { "" },
            ),
        };

        println!(
            "{:<40} {:>7} KB  {:<40} {}",
            name(path),
            bytes.len() / 1024,
            shape,
            describe(&verdict)
        );
        refused |= !verdict.passes();
    }

    if refused { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}

fn name(path: &std::path::Path) -> String {
    path.file_name().unwrap_or(path.as_os_str()).to_string_lossy().into_owned()
}

/// English only, and only here. Every sentence a charter reads comes from the
/// translated table instead; a debugging aid that needed fourteen translations
/// would not be worth having.
fn describe(verdict: &Verdict) -> String {
    match verdict {
        Verdict::Ok => "OK".to_owned(),
        Verdict::TooLarge { size_mb, limit_mb } => {
            format!("VIDEO_TOO_LARGE ({size_mb} MB, limit {limit_mb} MB)")
        }
        Verdict::Invalid => "INVALID_VIDEO".to_owned(),
        Verdict::FormatUnsupported { codec } => format!("VIDEO_FORMAT_UNSUPPORTED ({codec})"),
        Verdict::ResolutionTooHigh { width, height, max_width, max_height } => format!(
            "VIDEO_RESOLUTION_TOO_HIGH ({width}x{height}, limit {max_width}x{max_height} of pixels)"
        ),
        Verdict::FpsTooHigh { fps, limit } => format!("VIDEO_FPS_TOO_HIGH ({fps}, limit {limit})"),
        Verdict::HasAudio => "VIDEO_HAS_AUDIO".to_owned(),
    }
}
