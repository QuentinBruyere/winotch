mod notch;
mod platform;
mod tray;

use tauri::{Manager, WindowEvent};

pub const NOTCH_LABEL: &str = "notch";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let window = app
                .get_webview_window(NOTCH_LABEL)
                .expect("notch window is declared in tauri.conf.json");
            platform::prepare_overlay(&window)?;
            notch::place_on_primary_monitor(&window)?;
            window.show()?;

            tray::create(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != NOTCH_LABEL {
                return;
            }
            // On a DPI change the window still has its old size when this event fires,
            // and Windows then applies its own suggested rect. Re-center afterwards,
            // once the event loop is done with the resize: window calls made from
            // another thread are queued on the event loop instead of running inline.
            if matches!(
                event,
                WindowEvent::ScaleFactorChanged { .. } | WindowEvent::Resized(_)
            ) && let Some(w) = window.app_handle().get_webview_window(NOTCH_LABEL)
            {
                std::thread::spawn(move || {
                    if let Err(e) = notch::place_on_primary_monitor(&w) {
                        log::warn!("failed to reposition notch: {e}");
                    }
                });
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running winotch");
}
