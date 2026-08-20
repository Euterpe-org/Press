//! The four limits, mirrored from the server.
//!
//! The authority is `Euterpe.Web/backend/app/utils/chart_constants.py`. This is
//! the THIRD copy of these numbers (the second being the browser's, in
//! `frontend/src/utils/videoProbe.ts`) and copies drift, so it is pinned:
//! `tools/golden.py --check` reads the server's own constants and fails the
//! build if any of them has moved. Changing a number here without changing it
//! there does not produce a disagreement in the field -- it produces a red
//! build.
//!
//! Everything in this module is a value the server chose. Nothing here is a
//! Press decision.

/// 80 MB. The one limit that costs nothing to measure, and the one the other
/// three exist to protect.
pub const MAX_BYTES: u64 = 80 * 1024 * 1024;

/// "2K", enforced as a PIXEL COUNT rather than as a height. The live library
/// holds portrait uploads (1080x1920) that a height rule refuses while passing
/// the identical 1920x1080, and decoding does not care which side is tall. The
/// two reference dimensions are kept so a rejection can name a shape a person
/// recognises; the product is what is enforced.
pub const MAX_WIDTH: u32 = 2560;
pub const MAX_HEIGHT: u32 = 1440;
pub const MAX_PIXELS: u64 = MAX_WIDTH as u64 * MAX_HEIGHT as u64;

pub const MAX_FPS: u32 = 60;

/// Codecs the mod can play on every machine: AV1 through its own bundled dav1d,
/// H.264/H.265 through the game's Unity player. Anything else is a black screen
/// for some fraction of players, so it is refused at the door rather than
/// discovered in-game.
pub const ALLOWED_CODECS: [&str; 3] = ["h264", "h265", "av1"];

/// How far a background video may outrun its music before the tail is waste.
///
/// **This is not a gate limit and Press must never present it as one.** K ruled
/// it out of the gate on 2026-08-17: judging it needs two files at once, which
/// misbehaves in both directions on a server. Press has both files only when the
/// user hands them over, and even then the tail is an OFFER, worded as "players
/// never see this part", never as "your file is invalid".
pub const OVERRUN_ALLOWANCE_SECONDS: f64 = 10.0;

pub fn codec_allowed(codec: Option<&str>) -> bool {
    matches!(codec, Some(c) if ALLOWED_CODECS.contains(&c))
}
