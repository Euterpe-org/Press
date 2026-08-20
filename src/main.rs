//! Two ways in, and the same answer from both.
//!
//! Started with no arguments, Press opens a window: that is the product. Given
//! files, it answers from the terminal instead -- which is how a batch gets
//! checked without a drag-and-drop each, and how a release can be proved to
//! agree with the server rather than only the test harness doing so.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use euterpe_press::app::Press;
use euterpe_press::gate::{self, Verdict};
use euterpe_press::i18n::{Lang, text};
use euterpe_press::plan::Trim;
use euterpe_press::standardise::{self, Outcome, Step, Treatment};

fn main() -> ExitCode {
    let mut terminal = None;
    let mut files: Vec<PathBuf> = Vec::new();
    for arg in std::env::args_os().skip(1) {
        match arg.to_str() {
            Some("--check") => terminal = Some(false),
            Some("--fix") => terminal = Some(true),
            _ => files.push(PathBuf::from(arg)),
        }
    }

    // Bare file arguments open the WINDOW with that file loaded, because that is
    // what dropping a video on the exe -- or opening one "with" Press -- means to
    // the person doing it. A release build has no console attached
    // (`windows_subsystem = "windows"`), so answering that gesture on stdout
    // would print into nowhere and look like nothing happened at all.
    let Some(repair_them) = terminal else {
        return match window(files.into_iter().next()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("the window could not be opened: {error}");
                ExitCode::FAILURE
            }
        };
    };

    if files.is_empty() {
        eprintln!("usage: euterpe-press [--check | --fix] <video.mp4> [...]");
        eprintln!("       with no flag and no files, opens the window");
        return ExitCode::from(2);
    }

    let mut bad = false;
    for path in &files {
        bad |= if repair_them {
            repair(path)
        } else {
            report(path)
        };
    }
    if bad {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn window(open: Option<PathBuf>) -> eframe::Result {
    let lang = Lang::from_system();
    let ffmpeg = euterpe_press::encoder::resolve();

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([720.0, 860.0])
            .with_min_inner_size([560.0, 640.0])
            .with_drag_and_drop(true)
            .with_title(text::WINDOW_TITLE.get(lang)),
        ..Default::default()
    };

    eframe::run_native(
        text::WINDOW_TITLE.get(lang),
        options,
        Box::new(move |cc| {
            let mut press = Press::new(&cc.egui_ctx, lang, ffmpeg);
            if let Some(path) = open {
                press.open(&path);
            }
            Ok(Box::new(press))
        }),
    )
}

/// Read-only: what would the server say?
fn report(path: &Path) -> bool {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("{:<40} could not be read: {error}", name(path));
            return true;
        }
    };

    let (facts, verdict) = gate::judge(&bytes);
    println!(
        "{:<40} {:>7} KB  {:<40} {}",
        name(path),
        bytes.len() / 1024,
        shape(facts.as_ref()),
        describe(&verdict)
    );
    !verdict.passes()
}

/// Produce a file the server accepts, beside the source.
fn repair(path: &Path) -> bool {
    let output = path.with_file_name("video.mp4");
    if output == path {
        println!("{:<40} refusing to overwrite the source", name(path));
        return true;
    }

    let source_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    // The SAME encoder the window uses, resolved the same way. Taking whatever
    // was on PATH here would let `--fix` and the button produce different files
    // from one build, which is precisely the drift that shipping our own
    // encoder exists to prevent.
    let Some(ffmpeg) = euterpe_press::encoder::resolve() else {
        println!("{:<40} no encoder available", name(path));
        return true;
    };

    // The window owns a real one and wires it to a button. Nothing cancels a
    // command-line run, so this token is never set.
    let cancel = euterpe_press::ffmpeg::Cancel::new();

    let mut last = String::new();
    let outcome =
        standardise::standardise(&ffmpeg, path, &output, Trim::default(), &cancel, |step| {
            let line = match step {
                Step::Reading => "reading".to_owned(),
                Step::Copying(p) => format!("stream copy {}", percent(p.and_then(|p| p.fraction))),
                Step::Encoding {
                    crf,
                    attempt,
                    progress,
                } => format!(
                    "av1 crf {crf}{} {}",
                    if attempt > 1 {
                        format!(" (attempt {attempt})")
                    } else {
                        String::new()
                    },
                    percent(progress.and_then(|p| p.fraction)),
                ),
                Step::Verifying => "verifying".to_owned(),
            };
            if line != last {
                print!("\r{:<40} {line:<32}", name(path));
                let _ = std::io::Write::flush(&mut std::io::stdout());
                last = line;
            }
        });

    print!("\r{:<40} ", name(path));
    match outcome {
        Outcome::AlreadyStandard => {
            println!("{:<32} already standard, nothing written", "");
            false
        }
        Outcome::Cancelled => {
            println!("{:<32} cancelled, nothing written", "");
            true
        }
        Outcome::Failed(reason) => {
            println!("FAILED: {reason}");
            true
        }
        Outcome::Produced(produced) => {
            let mut did: Vec<String> = Vec::new();
            match produced.treatment {
                Treatment::StreamCopy => did.push("stream copy".to_owned()),
                Treatment::Av1 { crf } => did.push(format!("av1 crf {crf}")),
                Treatment::KeptOwnCodec { av1_ratio } => did.push(format!(
                    "kept {}, av1 came back at {:.0}%",
                    produced.facts.codec.unwrap_or("?"),
                    av1_ratio * 100.0
                )),
            }
            if produced.audio_dropped {
                did.push("audio dropped".to_owned());
            }
            if let Some((w, h)) = produced.scaled_to {
                did.push(format!("scaled to {w}x{h}"));
            }
            if let Some(fps) = produced.frame_rate_clamped {
                did.push(format!("{fps} fps"));
            }
            if let Some(trim) = produced.trimmed_to {
                did.push(format!("trimmed to {trim:.0}s"));
            }
            if produced.av1_cancelled {
                did.push("av1 comparison cancelled".to_owned());
            }
            println!(
                "{:>7} KB -> {:>4.0}%  {}",
                source_bytes / 1024,
                produced.ratio(source_bytes) * 100.0,
                did.join(", ")
            );
            false
        }
    }
}

fn percent(fraction: Option<f64>) -> String {
    fraction.map_or_else(|| "…".to_owned(), |f| format!("{:.0}%", f * 100.0))
}

fn name(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

fn shape(facts: Option<&gate::Facts>) -> String {
    match facts {
        None => "unreadable".to_owned(),
        Some(f) => format!(
            "{} {}x{} {:.2}fps {:.1}s{}",
            f.codec.unwrap_or(if f.raw_fourcc.is_empty() {
                "?"
            } else {
                &f.raw_fourcc
            }),
            f.width,
            f.height,
            f.fps,
            f.duration,
            if f.has_audio { " +audio" } else { "" },
        ),
    }
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
        Verdict::ResolutionTooHigh {
            width,
            height,
            max_width,
            max_height,
        } => format!(
            "VIDEO_RESOLUTION_TOO_HIGH ({width}x{height}, limit {max_width}x{max_height} of pixels)"
        ),
        Verdict::FpsTooHigh { fps, limit } => format!("VIDEO_FPS_TOO_HIGH ({fps}, limit {limit})"),
        Verdict::HasAudio => "VIDEO_HAS_AUDIO".to_owned(),
    }
}
