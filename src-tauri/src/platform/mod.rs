//! OS-specific tweaks for the notch window. Each OS gets its own module.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::{
    bring_to_front, foreground_class, fullscreen_on_notch_screen, prepare_overlay,
    restrict_to_shape, set_cursor_resistance, set_resistance_rect, set_resistance_strength,
    take_breakthrough,
};

#[cfg(not(windows))]
pub fn prepare_overlay(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
    Ok(())
}

/// Not implemented yet outside Windows: the notch never auto-hides there.
#[cfg(not(windows))]
pub fn fullscreen_on_notch_screen(_notch: &tauri::WebviewWindow) -> bool {
    false
}

/// Outside Windows the window itself is resized to the notch shape for now.
#[cfg(not(windows))]
pub fn restrict_to_shape(
    window: &tauri::WebviewWindow,
    _x: i32,
    _y: i32,
    width: i32,
    height: i32,
) -> tauri::Result<()> {
    window.set_size(tauri::PhysicalSize::new(width as u32, height as u32))?;
    crate::notch::place(window)
}

/// Cursor resistance needs a system-wide mouse hook: Windows only for now.
#[cfg(not(windows))]
pub fn set_cursor_resistance(_active: bool) {}

#[cfg(not(windows))]
pub fn set_resistance_rect(_rect: crate::resistance::Rect) {}

#[cfg(not(windows))]
pub fn set_resistance_strength(_strength: crate::resistance::Strength) {}

#[cfg(not(windows))]
pub fn bring_to_front(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.set_focus()
}

#[cfg(not(windows))]
pub fn take_breakthrough() -> Option<crate::resistance::Breakthrough> {
    None
}

#[cfg(not(windows))]
pub fn foreground_class() -> String {
    String::new()
}
