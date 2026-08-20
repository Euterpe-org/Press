//! Where the intermediate files live.
//!
//! Press now writes several files per run -- Phase A's lossless copy, and one
//! per encode Phase B tries -- and none of them is the thing the charter asked
//! for. They go to a directory Press owns, under random names, and the charter
//! is asked where to put the result only once, at the end, when there is a
//! result worth putting anywhere.
//!
//! That is a change of principle, not of plumbing. The old flow asked for a
//! destination BEFORE the work started, which meant asking someone to name a
//! file that might never exist -- and then writing `video.mp4` beside their
//! source, over whatever was already called that.
//!
//! # Cleanup is structural
//!
//! Dropping a [`Workspace`] deletes its directory. Abandoning a run drops it,
//! loading another file drops it, closing the window drops it. There is no
//! "remember to clean up" anywhere, because there is no path that can forget.
//!
//! A crash can still leave one behind, so [`sweep`] removes stale directories at
//! startup -- the same shape as [`crate::encoder::sweep`], and for the same
//! reason: a temp file nobody will ever look at again is still the user's disk.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Directories older than this are from a run that will never finish.
///
/// Generous, because the only cost of being wrong in this direction is a
/// directory that survives one extra session, while being wrong in the other
/// direction deletes the files a RUNNING Press is working on. A day is far
/// longer than any single encode and far shorter than "forever".
const STALE_AFTER: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);

fn root() -> PathBuf {
    std::env::temp_dir().join("Euterpe.Press")
}

/// A directory that exists as long as this value does.
#[derive(Debug)]
pub struct Workspace {
    dir: PathBuf,
}

impl Workspace {
    /// Make a fresh one. Fails only if the temp directory itself is unusable,
    /// which is not a condition Press can work around.
    pub fn open() -> std::io::Result<Self> {
        // No random number generator is linked, and none is needed: the pair
        // (process, instant) is unique across every Press that could be running,
        // and the counter separates workspaces within one process.
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = root().join(format!("work-{:x}-{stamp:x}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// A path inside the workspace. The file need not exist.
    pub fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        // Best effort: a locked file is not worth a panic on the way out, and
        // `sweep` will find it next time.
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Delete workspaces left behind by a Press that did not exit cleanly.
///
/// Best effort throughout. Anything still in use fails to delete and is left
/// alone, which is the correct outcome.
pub fn sweep() {
    let Ok(entries) = std::fs::read_dir(root()) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !entry.file_name().to_string_lossy().starts_with("work-") {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|m| now.duration_since(m).ok())
            .is_some_and(|age| age > STALE_AFTER);
        if stale {
            let _ = std::fs::remove_dir_all(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point: the directory's lifetime is the value's lifetime, so no
    /// code path can forget to clean up because no code path does the cleaning.
    #[test]
    fn dropping_a_workspace_takes_its_files_with_it() {
        let dir = {
            let work = Workspace::open().expect("a workspace");
            let file = work.file("something.mp4");
            std::fs::write(&file, b"not really an mp4").expect("write");
            assert!(file.exists());
            work.dir().to_path_buf()
        };
        assert!(!dir.exists(), "the directory outlived its Workspace");
    }

    /// Two workspaces alive at once must not share a directory -- one of them
    /// dropping would delete the other's files out from under it.
    #[test]
    fn two_workspaces_never_collide() {
        let a = Workspace::open().expect("a");
        let b = Workspace::open().expect("b");
        assert_ne!(a.dir(), b.dir());

        std::fs::write(a.file("x"), b"a").expect("write a");
        drop(b);
        assert!(a.file("x").exists(), "dropping b removed a's file");
    }
}
