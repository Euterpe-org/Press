//! Getting the encoder onto disk so it can be run.
//!
//! Press ships as one file. The encoder rides inside it, gzipped, and is
//! unpacked on first use to a cache file whose NAME is its content identity --
//! `ffmpeg-{stamp}.exe`. That is the doctrine Euterpe.Mod already uses for its
//! embedded natives, and the properties are the ones that matter here too:
//!
//! * A new build cannot collide with an older instance's still-locked file,
//!   because the bytes are different and so is the name.
//! * A stale build of the same length cannot be loaded in place of this one,
//!   because the wrong bytes simply live somewhere else.
//! * Two copies of Press running at once cannot corrupt each other: same name
//!   means same bytes, so whichever finished first is a file both can use.
//!
//! The length check on an existing file guards only a truncated write from a
//! crashed extraction. It is not an integrity check and does not pretend to be.
//!
//! # It is a CACHE, and it lives where caches live
//!
//! `%LOCALAPPDATA%\Euterpe.Press\`, not `%TEMP%`. The distinction is not
//! bookkeeping. `%TEMP%` means "deletable at any moment", and Windows means it:
//! Storage Sense and Disk Cleanup both target it on a schedule nobody sets. This
//! file is the opposite of that -- named by its own content, written once,
//! reused by every run for the life of the build, and worth keeping precisely
//! because unpacking it again costs the charter a wait they already paid for.
//!
//! It also stops Press matching the shape of a dropper. Unpacking an executable
//! into `%TEMP%` and running it is the single most heavily weighted pattern in
//! most heuristic scanners, and Press has to do the unpacking-and-running part
//! by design. Doing it in the folder Windows documents for cached program data
//! is both correct and one fewer reason to be stopped.
//!
//! The intermediates a RUN produces are genuinely temporary and stay in
//! `%TEMP%`, under [`crate::workspace`]. Two kinds of file, two lifetimes, two
//! directories, each named in exactly one place.
//!
//! **A build with no embedded encoder falls back to PATH.** That is for
//! development, and it is a compile-time branch rather than a runtime one: a
//! shipped Press always has its own, because a machine's stray ffmpeg changing
//! what Press produces is exactly the drift that using the server's recipe
//! exists to prevent.

use std::path::PathBuf;

use crate::ffmpeg::Ffmpeg;

/// Where the unpacked encoder is kept.
///
/// `%LOCALAPPDATA%\Euterpe.Press\` -- the folder Windows documents for per-user,
/// per-machine data that is not worth roaming and not safe to lose, which is
/// exactly what a content-addressed cache of an executable is.
///
/// Read from the environment rather than through `SHGetKnownFolderPath`. The
/// variable is set by the session for every process Windows starts, and reaching
/// for the shell API would mean another `windows-sys` feature and a COM-flavoured
/// call to learn something already on hand. If it is somehow absent, the temp
/// directory is a worse home but a working one, and losing a cache is not worth
/// refusing to run over.
pub fn cache_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Euterpe.Press")
}

#[cfg(embedded_encoder)]
mod embedded {
    use std::io::Read;
    use std::path::PathBuf;

    pub const PACKED: &[u8] = include_bytes!(env!("PRESS_FFMPEG_GZ"));
    pub const STAMP: &str = env!("PRESS_FFMPEG_STAMP");
    /// The uncompressed length, known at build time so a truncated extraction
    /// can be spotted without decompressing again.
    pub const BYTES: usize = {
        // `env!` gives a string; parsing it in a const fn keeps the number and
        // the bytes from ever drifting apart.
        let raw = env!("PRESS_FFMPEG_BYTES").as_bytes();
        let mut value = 0usize;
        let mut i = 0;
        while i < raw.len() {
            value = value * 10 + (raw[i] - b'0') as usize;
            i += 1;
        }
        value
    };

    pub fn unpack() -> std::io::Result<Vec<u8>> {
        let mut out = Vec::with_capacity(BYTES);
        flate2::read::GzDecoder::new(PACKED).read_to_end(&mut out)?;
        Ok(out)
    }

    /// Best-effort housekeeping: superseded builds' cache files would otherwise
    /// accumulate one per release. A file a live instance still holds open just
    /// stays for its owner.
    pub fn sweep(dir: &std::path::Path, keep: &PathBuf) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_ours = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("ffmpeg-") && n.ends_with(".exe"));
            if is_ours && path != *keep {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

/// Why there is no encoder to run.
///
/// TWO CAUSES, and they ask completely different things of the person reading
/// about them. Reporting both as "this build has no encoder" told somebody whose
/// antivirus had just quarantined the unpacked file that their download was
/// broken -- so they downloaded it again, and again, and it was never going to
/// be any different.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Absent {
    /// This build carries none and none was on PATH either. Development only:
    /// a release always carries its own.
    NotCarried,
    /// It IS in the executable, and it could not be put on disk. The causes are
    /// somebody else's and all of them are actionable -- a full disk, a
    /// read-only `TEMP`, or an antivirus that took the file between the write
    /// and the rename, which is the ordinary fate of an unsigned tool that
    /// extracts an executable.
    CouldNotUnpack(String),
}

/// The encoder this build will run.
///
/// Called once at startup. The window renders a plain sentence rather than
/// failing when the button is pressed: a tool that looks ready and then cannot
/// work is worse than one that says so on arrival.
pub fn resolve() -> Result<Ffmpeg, Absent> {
    #[cfg(embedded_encoder)]
    {
        // Never falling through to PATH. A release quietly running somebody
        // else's encoder is the one thing carrying our own exists to prevent,
        // and it would produce a file that differs from the server's for
        // reasons nobody could see.
        extract()
            .map(Ffmpeg::at)
            .map_err(|error| Absent::CouldNotUnpack(error.to_string()))
    }

    #[cfg(not(embedded_encoder))]
    {
        // Development only. A shipped build never reaches this.
        let usable = crate::ffmpeg::quietly(std::path::Path::new("ffmpeg"))
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success());
        usable.then(Ffmpeg::on_path).ok_or(Absent::NotCarried)
    }
}

#[cfg(embedded_encoder)]
fn extract() -> std::io::Result<std::path::PathBuf> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("ffmpeg-{}.exe", embedded::STAMP));

    let present = std::fs::metadata(&path).is_ok_and(|m| m.len() as usize == embedded::BYTES);
    if !present {
        let bytes = embedded::unpack()?;
        // Written beside and renamed into place, so a reader can never open a
        // half-written file: the name appears only once the bytes are all there.
        // Renaming is atomic within a directory, and a parallel instance doing
        // the identical thing simply wins the race harmlessly -- same name means
        // same content.
        let staging = dir.join(format!(
            "ffmpeg-{}.{}.part",
            embedded::STAMP,
            std::process::id()
        ));
        std::fs::write(&staging, &bytes)?;
        if let Err(error) = std::fs::rename(&staging, &path) {
            let _ = std::fs::remove_file(&staging);
            // Someone else finished first, and their file is ours too.
            if !std::fs::metadata(&path).is_ok_and(|m| m.len() as usize == embedded::BYTES) {
                return Err(error);
            }
        }
    }

    embedded::sweep(&dir, &path);
    Ok(path)
}

#[cfg(test)]
mod tests {
    /// Whether a build carries its own encoder is a property of the build, and
    /// the two branches must not both compile away to nothing.
    #[test]
    fn resolving_an_encoder_never_panics() {
        // Either answer is valid: a dev checkout with no vendor/ffmpeg.exe and
        // no ffmpeg on PATH legitimately has none.
        let _ = super::resolve();
    }

    /// Named, so a person browsing what this program left on their disk can tell
    /// what it is -- and kept OUT of the scratch directory, because the two hold
    /// different kinds of file with different lifetimes and Windows deletes one
    /// of them on a schedule nobody sets.
    #[test]
    fn the_cache_lives_apart_from_the_scratch_and_says_whose_it_is() {
        let cache = super::cache_dir();
        assert!(
            cache.ends_with("Euterpe.Press"),
            "a person should be able to tell what put this here: {}",
            cache.display()
        );
        assert_ne!(
            cache,
            crate::workspace::root(),
            "the encoder is a cache and the workspaces are scratch; \
             sharing a directory puts the cache somewhere Windows empties"
        );
    }
}
