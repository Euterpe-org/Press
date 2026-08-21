//! Compresses the encoder into the binary, if this build has one to carry.
//!
//! A release of Press ships its own ffmpeg so that a machine's stray one can
//! never change what it produces -- two builds disagreeing about a chart is
//! exactly what using the server's recipe exists to prevent. CI puts a purpose-
//! built, minimal ffmpeg at `vendor/ffmpeg.exe` and this squeezes it into the
//! executable.
//!
//! A checkout without that file still builds and still runs; it simply has no
//! encoder of its own and falls back to PATH, which is what a developer wants
//! and what `cargo test` needs. The window says so plainly rather than failing
//! at the moment somebody presses the button.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Who Press says it is, in the fields Explorer reads.
///
/// A charter is being asked to download an unsigned executable from a chat
/// group and run it. With these blank -- which they were -- its Properties page
/// is entirely empty, which is what an unidentified blob looks like to a
/// cautious person and to a heuristic scanner alike.
///
/// `FileVersion` and `ProductVersion` are not here because `winresource` takes
/// them from `CARGO_PKG_VERSION`. That is the point: the manifest is the single
/// place a version number exists, the binary quotes it in its own header and in
/// the crash dialog, and CI refuses a tag that disagrees with it.
const COMPANY: &str = "Euterpe";
const COPYRIGHT: &str = "© 2026 Euterpe";

fn main() {
    identify();
    embed_encoder();
}

/// The manifest and the version block. EVERY build, before anything that can
/// bail out early -- a release with a blank Properties page is the thing this
/// exists to prevent, so it must not be skipped by a checkout that happens to
/// have no encoder to embed.
fn identify() {
    println!("cargo:rerun-if-changed=app.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    check_manifest("app.manifest");

    let mut res = winresource::WindowsResource::new();
    res.set_manifest_file("app.manifest");
    res.set("ProductName", "Euterpe Press");
    res.set(
        "FileDescription",
        std::env::var("CARGO_PKG_DESCRIPTION")
            .unwrap_or_default()
            .as_str(),
    );
    res.set("CompanyName", COMPANY);
    res.set("LegalCopyright", COPYRIGHT);
    res.set("InternalName", "euterpe-press");
    res.set("OriginalFilename", "euterpe-press.exe");

    let Err(error) = res.compile() else {
        return;
    };
    // A DEBUG build may legitimately lack the resource compiler -- it comes with
    // the Windows SDK, and a checkout that only ever runs `cargo run` does not
    // need one. A RELEASE build is a thing that gets downloaded, so shipping it
    // nameless is not a trade to make quietly.
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        panic!(
            "the version resource could not be compiled, and a release must not ship without it: {error}"
        );
    }
    println!("cargo:warning=no version resource in this build ({error})");
}

/// The one way this project is likely to break its own manifest.
///
/// An XML comment may not contain a double hyphen, and this project's prose
/// uses one as an em dash in almost every paragraph it writes. Writing the
/// manifest's comments in the house style produced a file Windows rejects at
/// LOAD time: not a degraded feature, a hard loader failure before `main`,
/// reading "the application has failed to start because its side-by-side
/// configuration is incorrect (os error 14001)".
///
/// `rc.exe` does not read the XML, so nothing downstream of here objects. What
/// actually caught it was that the resource is compiled into the TEST binaries
/// too, and they stopped running -- which is a real guarantee and the reason CI
/// cannot ship a broken one, but it arrives as a baffling error from a test
/// runner rather than as a sentence about a manifest.
///
/// NOT a validator. It knows about the mistake that was made, so the second time
/// it is made it costs a line of output instead of an afternoon.
fn check_manifest(path: &str) {
    let Ok(text) = std::fs::read_to_string(path) else {
        panic!("{path} is missing, and the binary will not identify itself without it");
    };
    let mut rest = text.as_str();
    while let Some(open) = rest.find("<!--") {
        let after = &rest[open + 4..];
        let Some(close) = after.find("-->") else {
            panic!("{path}: an XML comment is never closed");
        };
        if after[..close].contains("--") {
            let line = text[..text.len() - rest.len() + open].lines().count();
            panic!(
                "{path}: line {line} opens a comment containing a double hyphen. \
                 XML forbids it, and Windows refuses the whole manifest at load \
                 time with os error 14001. Use a comma."
            );
        }
        rest = &after[close + 3..];
    }
}

fn embed_encoder() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source = root.join("vendor").join("ffmpeg.exe");
    println!("cargo:rerun-if-changed=vendor/ffmpeg.exe");
    println!("cargo:rustc-check-cfg=cfg(embedded_encoder)");

    let Ok(raw) = std::fs::read(&source) else {
        // No encoder in this build. Not an error: see the module docs.
        return;
    };

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("out dir"));
    let packed = out.join("ffmpeg.exe.gz");
    compress(&raw, &packed);

    // The cache file's NAME is its content identity, so a new build can never
    // reuse an older instance's still-locked file and a stale one of equal
    // length can never be loaded in its place. The same doctrine as
    // Euterpe.Mod's NativeCache, which names its extracted natives by hash.
    //
    // FNV-1a rather than SHA-256 because this is a cache key, not a signature:
    // the bytes never leave the executable they were compiled into, so there is
    // no attacker to collide with, and a real hash would mean a build
    // dependency for ten lines of arithmetic.
    let stamp = fnv1a(&raw);

    println!("cargo:rustc-cfg=embedded_encoder");
    println!("cargo:rustc-env=PRESS_FFMPEG_GZ={}", packed.display());
    println!("cargo:rustc-env=PRESS_FFMPEG_STAMP={stamp:016x}");
    println!("cargo:rustc-env=PRESS_FFMPEG_BYTES={}", raw.len());

    let packed_len = std::fs::metadata(&packed).map(|m| m.len()).unwrap_or(0);
    println!(
        "cargo:warning=embedded ffmpeg: {:.1} MB compressed to {:.1} MB",
        raw.len() as f64 / (1024.0 * 1024.0),
        packed_len as f64 / (1024.0 * 1024.0),
    );
}

fn compress(raw: &[u8], to: &Path) {
    let file = std::fs::File::create(to).expect("create the compressed encoder");
    let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::best());
    encoder.write_all(raw).expect("compress the encoder");
    encoder.finish().expect("finish compressing the encoder");
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
