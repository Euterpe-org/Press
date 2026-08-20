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
//! **A build with no embedded encoder falls back to PATH.** That is for
//! development, and it is a compile-time branch rather than a runtime one: a
//! shipped Press always has its own, because a machine's stray ffmpeg changing
//! what Press produces is exactly the drift that using the server's recipe
//! exists to prevent.

use crate::ffmpeg::Ffmpeg;

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

/// The encoder this build will run, or `None` if it has none and none is on
/// PATH either.
///
/// Called once at startup. The window renders a plain sentence rather than
/// failing when the button is pressed: a tool that looks ready and then cannot
/// work is worse than one that says so on arrival.
pub fn resolve() -> Option<Ffmpeg> {
    #[cfg(embedded_encoder)]
    {
        match extract() {
            Ok(path) => return Some(Ffmpeg::at(path)),
            Err(error) => {
                // Falling through to PATH would mean a release quietly running
                // somebody else's encoder, which is the one thing the embedded
                // copy exists to prevent. Say nothing was found instead.
                eprintln!("the bundled encoder could not be unpacked: {error}");
                return None;
            }
        }
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
        usable.then(Ffmpeg::on_path)
    }
}

#[cfg(embedded_encoder)]
fn extract() -> std::io::Result<std::path::PathBuf> {
    let dir = crate::workspace::root();
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

    #[test]
    fn the_cache_directory_is_named_after_this_program() {
        let dir = crate::workspace::root();
        assert!(
            dir.ends_with("Euterpe.Press"),
            "a person should be able to tell what put this here: {}",
            dir.display()
        );
    }
}
