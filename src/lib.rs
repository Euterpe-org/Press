//! Euterpe.Press -- the background-video formatter.
//!
//! A charter drags an MP4 in and is told, in their own language, whether
//! euterpe-org.com will accept it -- and if not, gets a file that it will.
//!
//! The library half is deliberately free of any UI: `gate` decides, and nothing
//! in it reads a file, opens a window or spawns a process. That is what lets the
//! verdict be tested against the server's own answers on seventeen fixtures
//! (`tests/gate.rs`) instead of against a screenshot.

pub mod gate;
pub mod plan;
