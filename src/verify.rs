//! Reading back what was written, and holding it to what was asked for.
//!
//! Both phases produce a file and neither trusts it. A tool that can emit a file
//! its own gate refuses has misled the charter, which is worse than not checking
//! at all -- so every intermediate and every candidate goes back through the same
//! probe the verdict came from.
//!
//! # Being too large is not an error
//!
//! This module draws one line, and it is the line the old code got wrong: a
//! [`Broken`] result means the file is not what was ASKED for -- unreadable,
//! wrong codec, wrong size, an audio track that survived, a length that moved.
//! Whether it would pass the gate comes back as [`Checked::verdict`], which is a
//! FACT about the file, not a complaint about it.
//!
//! Conflating the two is what made a 94 MB source stop dead at Phase A: the
//! lossless copy came back legal in every respect except its weight, that was
//! reported as a failure, and the encoder that would have fixed it never ran.
//! Phase A now expects to produce oversized files -- that is Phase B's entire
//! reason to exist.

use std::path::Path;

use crate::gate::{self, Facts, Verdict};

/// What the file was supposed to be.
#[derive(Debug, Clone)]
pub struct Expect {
    /// The frame size it must have.
    pub size: (u32, u32),
    /// The codec it must be in. `None` means "whatever the input had" and is
    /// only correct for a stream copy.
    pub codec: Option<&'static str>,
    /// How long it should run.
    pub seconds: f64,
    /// How far from `seconds` is still acceptable. A trim lands on a packet
    /// boundary, so it can sit a fraction of a second past the mark.
    pub tolerance: f64,
}

/// A file that is what it was supposed to be.
#[derive(Debug, Clone)]
pub struct Checked {
    pub bytes: u64,
    pub facts: Facts,
    /// Whether the gate would accept it AS IT STANDS. Data, not a judgement:
    /// Phase A routinely produces files that would be refused on size.
    pub verdict: Verdict,
}

/// The file is not what was asked for. A person has to look at these.
#[derive(Debug, Clone)]
pub struct Broken(pub String);

impl std::fmt::Display for Broken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Read the file back and hold it to `expect`.
pub fn judge(path: &Path, expect: &Expect) -> Result<Checked, Broken> {
    let broken = |reason: String| Err(Broken(reason));

    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => return broken(format!("the result could not be read back: {error}")),
    };
    let size = bytes.len() as u64;
    let Some(facts) = gate::probe(&bytes) else {
        return broken("the result is not a readable MP4".to_owned());
    };

    // Every path Press takes passes `-an`, so an audio track here means the
    // command that ran was not the command that was built.
    if facts.has_audio {
        return broken("the result still carries an audio track".to_owned());
    }

    if let Some(wanted) = expect.codec
        && facts.codec != Some(wanted)
    {
        return broken(format!(
            "the result is {}, not {wanted}",
            facts.codec.unwrap_or("an unrecognised codec"),
        ));
    }

    if (facts.width, facts.height) != expect.size {
        return broken(format!(
            "the result is {}x{}, expected {}x{}",
            facts.width, facts.height, expect.size.0, expect.size.1
        ));
    }

    // Checked in BOTH directions. Running long means a trim silently did not
    // take; running short means frames were dropped, which is the failure that
    // looks like success -- a smaller file that is missing the end of the video.
    if (facts.duration - expect.seconds).abs() > expect.tolerance {
        return broken(format!(
            "the result runs {:.2}s, expected {:.2}s",
            facts.duration, expect.seconds
        ));
    }

    let verdict = gate::check(size, Some(&facts));
    Ok(Checked {
        bytes: size,
        facts,
        verdict,
    })
}
