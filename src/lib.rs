//! Euterpe.Press -- the background-video formatter.
//!
//! A charter drags an MP4 in and is told, in their own language, whether
//! euterpe-org.com will accept it -- and if not, gets a file that it will.
//!
//! `gate` and `plan` are deliberately free of any UI and of any I/O: they take
//! bytes and facts and return verdicts and commands. That is what lets them be
//! tested against the server's own answers -- twenty-one fixtures and thirteen
//! planning scenarios -- rather than against a screenshot.

pub mod app;
pub mod crop;
pub mod elevation;
pub mod encoder;
pub mod ffmpeg;
pub mod fonts;
pub mod gate;
pub mod i18n;
pub mod phase_a;
pub mod phase_b;
pub mod plan;
pub mod preview;
pub mod theme;
pub mod timeline;
pub mod verify;
pub mod workspace;
