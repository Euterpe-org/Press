//! What Press says when Press is the thing that broke.
//!
//! A release build is `windows_subsystem = "windows"`: there is no console, so
//! the default panic message is written to a handle nobody owns and is gone. The
//! window then either vanishes mid-gesture, or -- if the panic was on a worker
//! thread -- sits there while the channel it was listening to goes quiet.
//!
//! `Cargo.toml` keeps unwinding on rather than aborting specifically so a panic
//! can still be shown. This is the part that keeps it around long enough.
//!
//! # Two panics, two different right answers
//!
//! **On a worker thread**, the window is alive and already handles the silence:
//! the channel disconnects and the drop screen says so. That is the better place
//! to be told -- still inside the program, with the file still loaded -- so the
//! reason is left in [`take`] for that screen to collect.
//!
//! **On the UI thread**, nothing downstream will get the chance. The window is
//! unwinding out from under itself and the next thing the charter sees is the
//! desktop. That one needs saying immediately and out of band.
//!
//! # This module does not know how to say anything
//!
//! It records, and it calls back. Showing is the BINARY's job, because a modal
//! dialog is only ever right when a window is about to disappear -- and a test
//! harness has no window, only a thread that panics whenever an assertion
//! fails. A library that could open one would hang the first CI job that failed
//! a test, holding a runner until it timed out with no output.
//!
//! Written the other way round first, and that is exactly what it did.

use std::sync::Mutex;

/// The last panic, waiting for a screen to show it.
static LAST: Mutex<Option<String>> = Mutex::new(None);

/// Never poisoned-panics. A panic *inside* a panic hook aborts the process,
/// which is precisely the silent death this module exists to prevent, so the
/// lock is taken in the one way that cannot cause one.
fn slot() -> std::sync::MutexGuard<'static, Option<String>> {
    LAST.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Install the hook. Once, before the window opens.
///
/// `last_words` is called for a panic on the CALLING thread and no other, on
/// the understanding that the caller is the thread whose death takes the
/// interface with it. Everything else is recorded for [`take`].
///
/// The default hook is kept and runs first, so `--check` and `--fix` runs and
/// CI logs stay exactly as loud as they were.
pub fn watch(last_words: impl Fn(&str) + Send + Sync + 'static) {
    let doomed = std::thread::current().id();
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        previous(info);
        let said = describe(info);
        if std::thread::current().id() == doomed {
            last_words(&said);
        }
        *slot() = Some(said);
    }));
}

/// The last panic's reason, if there has been one. Taken, not copied: a later
/// failure with no reason of its own must not inherit this one's and send
/// somebody to the wrong bug.
pub fn take() -> Option<String> {
    slot().take()
}

/// What broke and where, plus the build it broke in.
///
/// The version is here because it is the first thing anyone reading a report
/// has to ask and the last thing the person filing it thinks to include.
fn describe(info: &std::panic::PanicHookInfo<'_>) -> String {
    let what = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "something went wrong".to_owned());
    match info.location() {
        Some(at) => format!(
            "{what}\n{}:{} · {}",
            at.file(),
            at.line(),
            crate::app::VERSION
        ),
        None => format!("{what}\n{}", crate::app::VERSION),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc::{TryRecvError, channel};

    /// ONE test, because the hook and the slot are process-wide. Two of these
    /// running at once would take each other's reasons, and the failure would
    /// look like the bug this is here to catch.
    #[test]
    fn a_workers_last_words_outlive_the_worker() {
        static SHOWN: AtomicUsize = AtomicUsize::new(0);
        super::watch(|_| {
            SHOWN.fetch_add(1, Ordering::Relaxed);
        });
        let _ = super::take();

        // The sequence the window depends on: a worker dies, its channel
        // disconnects, and the reason is waiting where the drop screen looks.
        let (tx, inbox) = channel::<u8>();
        let worker = std::thread::spawn(move || {
            let _tx = tx;
            panic!("svt-av1 asked for a colour space that does not exist");
        });
        assert!(worker.join().is_err(), "the worker should have panicked");
        assert!(
            matches!(inbox.try_recv(), Err(TryRecvError::Disconnected)),
            "a dead worker must disconnect its channel"
        );

        let said = super::take().expect("the panic should have been recorded");
        assert!(
            said.contains("svt-av1 asked for a colour space that does not exist"),
            "the reason was lost: {said}"
        );
        assert!(
            said.contains(crate::app::VERSION),
            "a report needs the build it came from: {said}"
        );

        // Read once. A later failure with nothing to say must not inherit this.
        assert!(
            super::take().is_none(),
            "a reason must not be readable twice"
        );

        // And the out-of-band callback is for the DOOMED thread alone. A worker
        // panic reaching it would, in the real binary, be a modal dialog thrown
        // over a window that is still running and still able to explain itself.
        assert_eq!(
            SHOWN.load(Ordering::Relaxed),
            0,
            "a worker's panic must not be announced out of band"
        );
    }
}
