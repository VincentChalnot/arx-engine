//! Installing the window/taskbar icon — the one part of the GUI that has to
//! be written three times.
//!
//! minifb exposes `Window::set_icon(Icon)`, but `Icon` is not one type with
//! one constructor across platforms:
//!
//! - **Linux/X11**: `Icon: TryFrom<&[u64]>` (`#[cfg(target_os = "linux")]`
//!   in minifb) wraps a `_NET_WM_ICON` ARGB buffer — the format
//!   `window_icon::WINDOW_ICON` is generated in.
//! - **Windows**: the only constructor is `Icon::from_str`, a path to an
//!   `.ico` *file on disk* — no use to a single-file release binary, and
//!   minifb's implementation returns a pointer into a `Vec` it drops before
//!   returning, so it is unsound besides. The icon is built from the same
//!   embedded buffer here and handed to the window with `WM_SETICON`.
//! - **macOS**: nothing. minifb's `set_icon` is `unimplemented!()` and
//!   panics if called; a Mac app icon comes from the `.icns` in an `.app`
//!   bundle's `Info.plist`, which is a packaging concern, not a runtime one.
//!
//! Calling the Linux form unconditionally is what broke the Windows and
//! macOS release builds (`Icon: TryFrom<&[u64]>` is simply not implemented
//! there), hence this module.

use minifb::Window;

/// Set the embedded crest as `window`'s icon, where the platform allows it.
///
/// Best-effort by design: a window with the default icon is not worth
/// failing startup over, so every error path here is a silent no-op.
#[cfg(target_os = "linux")]
pub fn set_window_icon(window: &mut Window) {
    // `WINDOW_ICON` is a `static`, so the pointer minifb keeps inside `Icon`
    // stays valid for as long as it could possibly be read.
    if let Ok(icon) = minifb::Icon::try_from(&crate::window_icon::WINDOW_ICON[..]) {
        window.set_icon(icon);
    }
}

#[cfg(windows)]
pub fn set_window_icon(window: &mut Window) {
    use winapi::shared::minwindef::{LPARAM, WPARAM};
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::{SendMessageW, ICON_BIG, ICON_SMALL, WM_SETICON};

    let hwnd = window.get_window_handle() as HWND;
    if hwnd.is_null() {
        return;
    }
    let Some(icon) = create_icon() else {
        return;
    };
    // SAFETY: `hwnd` is the live window minifb just created and `icon` a
    // handle we own. The window keeps the icon for the rest of the process,
    // so it is deliberately never destroyed (freeing it here would leave the
    // window pointing at a dead handle).
    unsafe {
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, icon as LPARAM);
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as WPARAM, icon as LPARAM);
    }
}

/// Build an `HICON` from the generated ARGB buffer: a 32-bit top-down DIB
/// whose alpha channel carries the transparency (the GLFW/winit recipe).
///
/// The buffer is `[width, height, argb pixels...]` — X11's `_NET_WM_ICON`
/// layout, one `u64` slot per value, of which only the low 32 bits are used.
/// Reusing it keeps `assets/pixel/logo.xcf` the single source of truth for
/// the icon on both platforms.
#[cfg(windows)]
fn create_icon() -> Option<winapi::shared::windef::HICON> {
    use winapi::ctypes::c_void;
    use winapi::shared::minwindef::{DWORD, TRUE};
    use winapi::shared::windef::{HBITMAP, HGDIOBJ};
    use winapi::um::wingdi::{
        CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPV5HEADER, BI_BITFIELDS,
        DIB_RGB_COLORS,
    };
    use winapi::um::winuser::{CreateIconIndirect, GetDC, ReleaseDC, ICONINFO};

    let data = &crate::window_icon::WINDOW_ICON;
    let (w, h) = (data[0] as i32, data[1] as i32);
    let pixels = &data[2..];
    if w <= 0 || h <= 0 || pixels.len() != (w as usize) * (h as usize) {
        return None;
    }

    // SAFETY: plain GDI calls, each result null-checked before use. The DIB
    // is written through the pointer `CreateDIBSection` hands back, for
    // exactly the `w * h` pixels it was asked to allocate.
    unsafe {
        let mut header: BITMAPV5HEADER = std::mem::zeroed();
        header.bV5Size = std::mem::size_of::<BITMAPV5HEADER>() as DWORD;
        header.bV5Width = w;
        // Negative height = top-down rows, matching the buffer's row order.
        header.bV5Height = -h;
        header.bV5Planes = 1;
        header.bV5BitCount = 32;
        header.bV5Compression = BI_BITFIELDS;
        header.bV5RedMask = 0x00ff_0000;
        header.bV5GreenMask = 0x0000_ff00;
        header.bV5BlueMask = 0x0000_00ff;
        header.bV5AlphaMask = 0xff00_0000;

        let dc = GetDC(std::ptr::null_mut());
        let mut bits: *mut c_void = std::ptr::null_mut();
        let color = CreateDIBSection(
            dc,
            &header as *const BITMAPV5HEADER as *const BITMAPINFO,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        ReleaseDC(std::ptr::null_mut(), dc);
        if color.is_null() || bits.is_null() {
            return None;
        }
        // The masks above make each DIB pixel a 0xAARRGGBB word — the same
        // encoding the generator emits — so this is a straight copy.
        let dst = std::slice::from_raw_parts_mut(bits as *mut u32, pixels.len());
        for (d, s) in dst.iter_mut().zip(pixels) {
            *d = *s as u32;
        }

        // 1-bit AND mask, all zeros ("opaque"). It is ignored for a 32-bit
        // icon — the alpha channel is what shapes it — but
        // `CreateIconIndirect` still wants a bitmap, and `CreateBitmap`
        // leaves one built from a null pointer full of undefined bits.
        // Scan lines are WORD-aligned.
        let mask_bits = vec![0u8; (w as usize).div_ceil(16) * 2 * h as usize];
        let mask: HBITMAP = CreateBitmap(w, h, 1, 1, mask_bits.as_ptr() as *const c_void);
        if mask.is_null() {
            DeleteObject(color as HGDIOBJ);
            return None;
        }

        let mut info: ICONINFO = std::mem::zeroed();
        info.fIcon = TRUE;
        info.hbmMask = mask;
        info.hbmColor = color;
        let icon = CreateIconIndirect(&mut info);

        // CreateIconIndirect copies both bitmaps into the icon.
        DeleteObject(color as HGDIOBJ);
        DeleteObject(mask as HGDIOBJ);

        if icon.is_null() {
            None
        } else {
            Some(icon)
        }
    }
}

/// macOS (and anything else minifb has no icon API for): nothing to do —
/// `Window::set_icon` there is `unimplemented!()` and would panic.
#[cfg(not(any(target_os = "linux", windows)))]
pub fn set_window_icon(_window: &mut Window) {}
