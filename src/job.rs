//! The file, and what the charter decided about it.
//!
//! # Why this is one value
//!
//! These five things travel together through every screen and both phases, and
//! they used to be restated at each stop: five fields on the inspect screen,
//! five more on a `Held` struct built for the run, five more on the bench, and a
//! five-argument function to put them back. Every transition was a hand-written
//! copy, and a hand-written copy is a place to leave something out.
//!
//! Something was left out, twice, and it was the same thing both times. The tail
//! of the trim went missing on the way into Phase B (`keep_until: None`, written
//! because Phase A had already applied it) and again on the way back out to the
//! scrubber -- so the one screen reached by a card saying "make it shorter"
//! opened with the charter's sixty-second trim replaced by the whole source.
//!
//! Carried as one value there is no copy to get wrong. It moves.

use std::path::PathBuf;

use crate::crop;
use crate::gate::Facts;
use crate::plan::Trim;

/// A file Press has read, and every decision the charter has made about it.
#[derive(Debug, Clone)]
pub struct Job {
    /// The charter's own file. Never an intermediate -- a screen naming a
    /// random temp file tells nobody anything.
    pub source: PathBuf,
    pub bytes: u64,
    /// Always present. A file Press cannot read never becomes a job: it is
    /// refused at the door with a reason, because a screen built around a file
    /// nothing can be learnt about is a dead end dressed as a workspace.
    pub facts: Facts,

    // ── and what the charter asked for ────────────────────────────────────
    /// Which part of the source survives. Untouched by default: the opening is
    /// never moved unless the charter moves it.
    pub trim: Trim,
    /// The shape they asked for, if they asked. An OFFER, never applied on
    /// Press's own initiative -- the gate has no opinion about shape.
    pub crop: Option<crop::Choice>,
}

impl Job {
    /// Everything asked for that only a re-encode can carry out.
    ///
    /// Resolved against `frame` -- the picture of whatever is being encoded --
    /// rather than against the source, because Phase A may already have handed
    /// Phase B something else.
    pub fn request(&self, frame: (u32, u32)) -> Request {
        Request {
            opening_cut: self.trim.cut_opening,
            crop: self.crop.and_then(|choice| choice.window(frame)),
        }
    }
}

/// What the charter asked for, resolved and ready for the one encode that can
/// carry it out.
///
/// Both halves are here for the same reason: they rewrite the picture. The
/// opening cut needs `-ss` and `-ss` re-encodes; a crop rewrites every frame by
/// definition. Phase A can execute neither without costing a generation, so both
/// wait for Phase B and travel together to get there.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Request {
    /// Seconds taken off the front.
    pub opening_cut: f64,
    /// A rectangle by the time it arrives, not a shape still to be worked out.
    pub crop: Option<crop::Window>,
}

impl Request {
    /// Whether anything was asked for at all.
    pub fn is_empty(&self) -> bool {
        self.opening_cut <= 0.0 && self.crop.is_none()
    }
}
