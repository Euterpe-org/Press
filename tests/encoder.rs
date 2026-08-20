//! The embedded encoder makes it to disk intact.
//!
//! Press ships as one file with ffmpeg inside it. If the unpacking is wrong the
//! symptom is not a clear failure — it is an encoder that runs and produces
//! something subtly different, which is the one outcome the whole "use the
//! server's recipe" discipline exists to prevent. So the round trip is checked
//! against the bytes that went in.
//!
//! Skips itself in a checkout with no `vendor/ffmpeg.exe`, which is every
//! developer's, because there is then nothing embedded to verify.

#[cfg(embedded_encoder)]
mod embedded {
    use std::path::PathBuf;

    fn vendored() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/ffmpeg.exe")
    }

    /// Byte for byte. A near-miss here is an encoder that runs and lies.
    #[test]
    fn what_comes_out_is_what_went_in() {
        let expected = std::fs::read(vendored()).expect("the vendored encoder");
        let ffmpeg = euterpe_press::encoder::resolve().expect("an embedded build resolves one");

        let unpacked = std::fs::read(ffmpeg.binary()).expect("the unpacked encoder");
        assert_eq!(
            unpacked.len(),
            expected.len(),
            "unpacked to a different length"
        );
        assert!(unpacked == expected, "unpacked to different bytes");
    }

    /// The cache file is named for its CONTENT, so a new build cannot reuse an
    /// older instance's file and a stale one of equal length cannot stand in for
    /// it. Same doctrine as Euterpe.Mod's NativeCache.
    #[test]
    fn the_cache_file_is_named_for_its_contents() {
        let ffmpeg = euterpe_press::encoder::resolve().expect("an embedded build resolves one");
        let name = ffmpeg
            .binary()
            .file_name()
            .and_then(|n| n.to_str())
            .expect("a file name");

        assert!(name.starts_with("ffmpeg-"), "{name}");
        assert!(name.ends_with(".exe"), "{name}");
        assert_eq!(
            name.len(),
            "ffmpeg-".len() + 16 + ".exe".len(),
            "the stamp should be the full 64-bit hash: {name}"
        );
    }

    /// Resolving twice hands back the same file rather than unpacking again --
    /// and, more importantly, does not leave a second copy behind.
    #[test]
    fn resolving_twice_is_the_same_file_and_leaves_no_litter() {
        let first = euterpe_press::encoder::resolve().expect("resolved");
        let second = euterpe_press::encoder::resolve().expect("resolved again");
        assert_eq!(first.binary(), second.binary());

        let dir = first.binary().parent().expect("a cache directory");
        let strays: Vec<_> = std::fs::read_dir(dir)
            .expect("read the cache")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.ends_with(".part"))
            .collect();
        assert!(
            strays.is_empty(),
            "half-written files left behind: {strays:?}"
        );
    }
}

/// Without an embedded encoder the resolver must still answer rather than
/// panic: that is every development checkout, and `cargo test` runs there.
#[cfg(not(embedded_encoder))]
#[test]
fn a_build_without_an_encoder_still_answers() {
    let _ = euterpe_press::encoder::resolve();
}
