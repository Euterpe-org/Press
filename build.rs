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

fn main() {
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
