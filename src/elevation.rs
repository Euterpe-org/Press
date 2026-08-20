//! Is Press running as administrator, and does that matter?
//!
//! It matters for exactly one thing, and it is the first thing a charter will
//! try: **Windows silently blocks drag-and-drop into an elevated window.**
//! Explorer runs at medium integrity, an elevated Press runs at high, and UIPI
//! refuses to let a message cross that boundary. The whole visible result is a
//! forbidden cursor. Nothing is logged, nothing is said, and the obvious
//! conclusion is that the program is broken.
//!
//! Press has no reason to be elevated — it reads a video and writes one beside
//! it — so the answer is not to work around the block but to explain it. The
//! file picker is unaffected and still works, so the window can say "drag is
//! blocked, use the button, or reopen normally" and the charter is never left
//! guessing.
//!
//! # Why not `ChangeWindowMessageFilterEx`
//!
//! It is the usual advice for elevated windows that want drops, by allowing
//! `WM_DROPFILES`, `WM_COPYDATA` and `WM_COPYGLOBALDATA` through UIPI. winit
//! does not use `WM_DROPFILES` — it implements OLE's `IDropTarget` — and whether
//! the filter is enough for that path is not something this could verify without
//! a human dragging a file into an elevated window. Shipping Win32 code that
//! might do nothing, to fix a case that already has a working alternative, is
//! worse than a sentence that is true.

/// True when this process has an elevated token.
///
/// Anything unexpected answers `false`: the only consequence is that a warning
/// is not shown, which is exactly the state a non-elevated run is in anyway.
#[cfg(windows)]
pub fn is_elevated() -> bool {
    use std::mem::size_of;

    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return false;
        }

        let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            (&raw mut elevation).cast(),
            size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        );
        CloseHandle(token);

        ok != 0 && elevation.TokenIsElevated != 0
    }
}

#[cfg(not(windows))]
pub fn is_elevated() -> bool {
    false
}

#[cfg(test)]
mod tests {
    /// Both answers are legitimate — a developer's test run may be either — so
    /// what is asserted is that asking is safe and settled, not what it says.
    /// An unsafe Win32 call that mishandled its token would show up here as a
    /// crash rather than as a wrong boolean.
    #[test]
    fn asking_is_safe_and_stable() {
        let first = super::is_elevated();
        for _ in 0..64 {
            assert_eq!(super::is_elevated(), first, "the answer should not wander");
        }
        println!("running elevated: {first}");
    }
}
