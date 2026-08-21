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
use euterpe_press::job::{Job, Request};
use euterpe_press::plan::Trim;
use euterpe_press::workspace::Workspace;
use euterpe_press::{phase_a, phase_b};

fn main() -> ExitCode {
    // A Press that was killed mid-run leaves its workspace behind. Nothing will
    // ever look at those files again, and they are megabytes each.
    euterpe_press::workspace::sweep();

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
    // Before anything that could break. A release build has no console, so
    // without this a panic on THIS thread writes its explanation to a handle
    // nobody owns and the window simply disappears.
    //
    // Only on this path, and only from the binary. `--check` and `--fix` run in
    // a terminal that already shows the default hook's output, and a batch of a
    // hundred files must not stop on a dialog nobody is there to close. A panic
    // on a worker thread is not shown here at all -- the window survives it and
    // says so on the drop screen, which is a better place to be told.
    euterpe_press::bug::watch(move |said| last_words(lang, said));
    let ffmpeg = euterpe_press::encoder::resolve();

    // FIXED. Not resizable, not maximisable: the body is one canvas laid out
    // deliberately, and every size other than this one was a size nothing was
    // designed for.
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size(euterpe_press::app::WINDOW)
            .with_resizable(false)
            .with_maximize_button(false)
            .with_drag_and_drop(true)
            .with_title(euterpe_press::app::window_title(lang)),
        ..Default::default()
    };

    eframe::run_native(
        &euterpe_press::app::window_title(lang),
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

/// The window is unwinding out from under itself, so this is the last chance to
/// say anything at all.
///
/// The first thing it says is the thing they will actually worry about. Press
/// never writes to the file it was given -- every intermediate goes to a temp
/// directory that sweeps itself -- and somebody watching their only export of a
/// three-hour edit disappear needs to hear that before they hear anything else.
fn last_words(lang: Lang, said: &str) {
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(text::WINDOW_TITLE.get(lang))
        .set_description(format!(
            "{}\n\n{}\n\n{said}",
            text::BUG_HEADLINE.get(lang),
            text::BUG_YOUR_FILE_IS_SAFE.get(lang),
        ))
        .show();
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
    // Named after the SOURCE, so a folder of ten videos produces ten results
    // rather than each one silently destroying the last. The window asks where
    // to put it; nothing here can ask, so nothing here overwrites -- a batch
    // that eats its own earlier output is worse than one that stops and says
    // the name is taken.
    let output = path.with_file_name(euterpe_press::job::result_name(path));

    if output == path {
        println!("{:<40} refusing to overwrite the source", name(path));
        return true;
    }
    if output.exists() {
        println!(
            "{:<40} {} already exists, leaving it alone",
            name(path),
            name(&output)
        );
        return true;
    }

    let source_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    // The SAME encoder the window uses, resolved the same way. Taking whatever
    // was on PATH here would let `--fix` and the button produce different files
    // from one build, which is precisely the drift that shipping our own
    // encoder exists to prevent.
    let Ok(ffmpeg) = euterpe_press::encoder::resolve() else {
        println!("{:<40} no encoder available", name(path));
        return true;
    };

    // The window owns a real one and wires it to a button. Nothing cancels a
    // command-line run, so this token is never set.
    let cancel = euterpe_press::ffmpeg::Cancel::new();
    let work = match Workspace::open() {
        Ok(work) => work,
        Err(error) => {
            println!("{:<40} no temporary directory: {error}", name(path));
            return true;
        }
    };

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            println!("{:<40} could not be read: {error}", name(path));
            return true;
        }
    };
    let Some(facts) = euterpe_press::gate::probe(&bytes) else {
        println!("{:<40} not a readable MP4", name(path));
        return true;
    };
    drop(bytes);

    let mut last = String::new();
    let mut say = |line: String| {
        if line != last {
            print!("\r{:<40} {line:<40}", name(path));
            let _ = std::io::Write::flush(&mut std::io::stdout());
            last = line;
        }
    };

    // Nothing is asked of anyone here, so the job carries no requests: the
    // batch tool formats what it is given and never crops or moves an opening.
    let job = Job {
        source: path.to_path_buf(),
        bytes: source_bytes,
        facts,
        trim: Trim::default(),
        crop: None,
    };

    // ── Phase A ──────────────────────────────────────────────────────────
    let settled = phase_a::run(&ffmpeg, &job, &work, &cancel, |step| {
        say(match step {
            phase_a::Step::Reading => "reading".to_owned(),
            phase_a::Step::Copying(p) => format!(
                "lossless pass {} {}",
                percent(p.and_then(|p| p.fraction)),
                written(p.map(|p| p.bytes)),
            ),
            phase_a::Step::Verifying => "verifying".to_owned(),
        });
    });
    let material = match settled {
        phase_a::Outcome::Settled(material) => material,
        phase_a::Outcome::Cancelled => {
            println!("\r{:<40} cancelled", name(path));
            return true;
        }
        phase_a::Outcome::Failed(reason) => {
            println!("\r{:<40} FAILED: {reason}", name(path));
            return true;
        }
    };

    let mut did: Vec<String> = material
        .applied
        .iter()
        .map(|applied| match applied {
            phase_a::Applied::AudioDropped => "audio dropped".to_owned(),
            phase_a::Applied::TailCut { now, .. } => format!("trimmed to {now:.0}s"),
        })
        .collect();

    // ── Phase B ──────────────────────────────────────────────────────────
    let Some(aim) = phase_b::aim(&material.facts, material.bytes, &Request::default()) else {
        // Already conforming AV1: nothing to win and a generation to lose.
        return deliver(path, material.path(), &output, source_bytes, &did);
    };

    let outcome = phase_b::run(
        &ffmpeg,
        &material,
        aim,
        &Request::default(),
        &work,
        &cancel,
        |step| {
            say(match step {
                phase_b::Step::Encoding {
                    crf,
                    attempt,
                    progress,
                } => format!(
                    "av1 crf {crf}{} {} {}",
                    if attempt > 1 {
                        format!(" (encode {attempt})")
                    } else {
                        String::new()
                    },
                    percent(progress.and_then(|p| p.fraction)),
                    written(progress.map(|p| p.bytes)),
                ),
                phase_b::Step::Recalibrating { from, to } => format!("crf {from} -> {to}"),
                phase_b::Step::Verifying => "verifying".to_owned(),
            });
        },
    );

    match outcome {
        phase_b::Outcome::Encoded(done) => {
            did.insert(0, format!("av1 crf {}", done.crf));
            if done.encodes > 1 {
                did.push(format!("{} encodes", done.encodes));
            }
            deliver(path, &done.path, &output, source_bytes, &did)
        }
        phase_b::Outcome::MaterialWins { av1_was } => {
            did.push(match av1_was {
                Some(was) => format!("av1 came back at {} KB, kept the copy", was / 1024),
                None => "av1 was heading past the source, kept the copy".to_owned(),
            });
            deliver(path, material.path(), &output, source_bytes, &did)
        }
        // The file is deliverable; only the crop or the opening cut is not.
        phase_b::Outcome::RequestDropped => {
            did.push("the requested change would have made it heavier, dropped".to_owned());
            deliver(path, material.path(), &output, source_bytes, &did)
        }
        phase_b::Outcome::StillTooLarge { smallest } => {
            println!(
                "\r{:<40} TOO LONG: smallest was {} MB, limit is {} MB",
                name(path),
                smallest.div_ceil(1024 * 1024),
                euterpe_press::gate::limits::MAX_BYTES.div_ceil(1024 * 1024),
            );
            true
        }
        phase_b::Outcome::Cancelled => {
            println!("\r{:<40} cancelled", name(path));
            true
        }
        phase_b::Outcome::Failed(reason) => {
            println!("\r{:<40} FAILED: {reason}", name(path));
            true
        }
    }
}

/// Copy the winner out of the workspace, which is about to be deleted.
fn deliver(source: &Path, from: &Path, to: &Path, source_bytes: u64, did: &[String]) -> bool {
    if let Err(error) = std::fs::copy(from, to) {
        println!("\r{:<40} FAILED: could not write: {error}", name(source));
        return true;
    }
    let after = std::fs::metadata(to).map(|m| m.len()).unwrap_or(0);
    println!(
        "\r{:<40} {:>7} KB -> {:>4.0}%  {}",
        name(source),
        source_bytes / 1024,
        if source_bytes > 0 {
            after as f64 / source_bytes as f64 * 100.0
        } else {
            0.0
        },
        did.join(", ")
    );
    false
}

fn percent(fraction: Option<f64>) -> String {
    fraction.map_or_else(|| "…".to_owned(), |f| format!("{:.0}%", f * 100.0))
}

/// How much has been written so far. The size is the thing being fought over,
/// so watching it grow is more use than watching the percentage.
fn written(bytes: Option<u64>) -> String {
    bytes.map_or_else(String::new, |b| {
        format!("{:.1} MB", b as f64 / (1024.0 * 1024.0))
    })
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
