//! Installing the window/taskbar icon — split per platform because minifb
//! exposes `Window::set_icon(Icon)`, but `Icon` is not one type with one
//! constructor across platforms:
//!
//! - **Linux/X11**: `Icon: TryFrom<&[u64]>` (`#[cfg(target_os = "linux")]`
//!   in minifb) wraps a `_NET_WM_ICON` ARGB buffer — the format
//!   `window_icon::WINDOW_ICON` is generated in.
//! - **Windows**: minifb's only constructor, `Icon::from_str`, wants a path
//!   to an `.ico` file on disk — no use to a single-file release binary,
//!   and its implementation returns a pointer into a `Vec` it drops before
//!   returning, so it is unsound besides. None of that is needed: the .exe
//!   already carries `assets/generated/keres.ico` as a PE resource (see
//!   `build.rs`), which Explorer, the taskbar and Alt-Tab pick up on their
//!   own with no runtime call — a window that doesn't call `set_icon` just
//!   keeps its process's icon.
//! - **macOS**: nothing at runtime either way. minifb's `set_icon` is
//!   `unimplemented!()` and panics if called; a Mac app icon comes from the
//!   `.icns` in an `.app` bundle's `Info.plist` (see
//!   `scripts/package_macos_app.sh`), a packaging concern, not a runtime one.
//!
//! Calling the Linux form unconditionally is what broke the Windows and
//! macOS release builds (`Icon: TryFrom<&[u64]>` is simply not implemented
//! there), hence this module.

use minifb::Window;

/// Set the embedded crest as `window`'s icon, where the platform needs it
/// done at runtime.
///
/// Best-effort by design: a window with the default icon is not worth
/// failing startup over, so the error path here is a silent no-op.
#[cfg(target_os = "linux")]
pub fn set_window_icon(window: &mut Window) {
    // `WINDOW_ICON` is a `static`, so the pointer minifb keeps inside `Icon`
    // stays valid for as long as it could possibly be read.
    if let Ok(icon) = minifb::Icon::try_from(&crate::window_icon::WINDOW_ICON[..]) {
        window.set_icon(icon);
    }
}

/// Windows and macOS both get their icon without a runtime call — see the
/// module docs above.
#[cfg(not(target_os = "linux"))]
pub fn set_window_icon(_window: &mut Window) {}
