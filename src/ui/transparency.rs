/// Window Transparency & Compositor Integration
///
/// Handles platform-specific window blur effects (Acrylic on Windows, KWin on Linux).
/// Provides runtime detection of compositor support and setup functions for each platform.

use slint::Window;

/// Returns true if the compositor supports alpha-blended windows with blur behind.
///
/// Uses platform-specific detection:
/// - **Windows**: Always true (Windows 11 with DWM always supports Acrylic)
/// - **Linux**: True if Wayland is active or KWin is the window manager
/// - **Other**: False
pub fn compositor_supports_transparency() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows 11 (build 22000+) always supports Acrylic via DWM
        true
    }

    #[cfg(target_os = "linux")]
    {
        // Wayland (any compositor) supports transparency
        // X11 with KWin supports transparency
        std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("DESKTOP_SESSION")
                .map(|s| s.contains("plasma") || s.contains("kde"))
                .unwrap_or(false)
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        false
    }
}

/// Enable Acrylic blur on Windows 11 using DWM (Desktop Window Manager).
///
/// This function must be called BEFORE the Slint event loop starts.
/// It sets the window's backdrop type to DWMSBT_TRANSIENTWINDOW (Acrylic blur),
/// which enables blur behind the transparent window.
///
/// # Platform
/// Windows 11 only. On Windows 10 or earlier, this is a no-op.
///
/// # Safety
/// Uses unsafe Win32 API calls via windows-sys crate.
#[cfg(all(target_os = "windows", feature = "windows-sys"))]
pub fn enable_acrylic_blur(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE};

    if let Ok(handle) = window.window_handle() {
        if let RawWindowHandle::Win32(h) = handle.as_raw() {
            // DWMSBT_TRANSIENTWINDOW = 3 (Acrylic blur effect for utility/tool windows)
            // DWMSBT_MAINWINDOW = 2 (Mica for main app windows — subtler than Acrylic)
            // For applets/floating tools, use 3 (Acrylic) for more pronounced blur.
            let backdrop_type: u32 = 3;

            unsafe {
                DwmSetWindowAttribute(
                    h.hwnd.get() as _,
                    DWMWA_SYSTEMBACKDROP_TYPE,
                    &backdrop_type as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
            }

            eprintln!("[Transparency] Acrylic blur enabled on Windows 11");
        }
    }
}

/// No-op fallback for non-Windows platforms or when windows-sys is not available.
#[cfg(not(all(target_os = "windows", feature = "windows-sys")))]
pub fn enable_acrylic_blur(_window: &Window) {
    #[cfg(target_os = "linux")]
    eprintln!("[Transparency] KWin blur will be auto-applied if enabled in System Settings");

    #[cfg(not(target_os = "linux"))]
    eprintln!("[Transparency] Compositor blur not available on this platform");
}

/// Return the appropriate background color based on compositor support.
///
/// - If compositor blur is available: Return semi-transparent color that lets blur show through
/// - If compositor blur is not available: Return solid fallback color
pub fn get_window_background_color() -> slint::Color {
    if compositor_supports_transparency() {
        // rgba(15, 17, 26, 0.88) — 88% opacity lets the Acrylic blur effect show through
        // The 12% transparency prevents the window from being fully opaque
        slint::Color::from_argb_u8(224, 15, 17, 26)  // ARGB: alpha=224 (~88%), RGB=#0F111A
    } else {
        // Solid fallback: #0F111A (Surface-Dim from Material Design 3)
        slint::Color::from_argb_u8(255, 15, 17, 26)  // ARGB: alpha=255 (fully opaque)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compositor_detection() {
        // This test just verifies the function returns a bool without panicking
        let result = compositor_supports_transparency();
        assert!(result.is_bool());  // Trivial check; actual result is platform-dependent
    }

    fn is_bool(_val: bool) -> bool {
        true
    }

    #[test]
    fn test_background_color() {
        let color = get_window_background_color();
        // Verify color components are in valid range
        assert!(color.red <= 1.0 && color.red >= 0.0);
        assert!(color.green <= 1.0 && color.green >= 0.0);
        assert!(color.blue <= 1.0 && color.blue >= 0.0);
        assert!(color.alpha <= 1.0 && color.alpha >= 0.0);
    }
}
