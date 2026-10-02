//! OS-specific tweaks for the notch window. Each OS gets its own module.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::prepare_overlay;

#[cfg(not(windows))]
pub fn prepare_overlay(_window: &tauri::WebviewWindow) -> tauri::Result<()> {
    Ok(())
}
