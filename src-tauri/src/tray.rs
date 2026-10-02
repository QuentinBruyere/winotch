use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

use crate::NOTCH_LABEL;

/// Tray menu: quick actions only, everything else lives in the settings window.
pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(
        app,
        "toggle",
        "Afficher / masquer le notch",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "settings", "Paramètres…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter winotch", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &settings,
            &toggle,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(
            app.default_window_icon()
                .cloned()
                .expect("bundle icon is configured"),
        )
        .tooltip("winotch")
        .menu(&menu)
        .show_menu_on_left_click(false)
        // Left click opens the settings, right click shows the menu.
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
                && let Err(e) = crate::settings::open(tray.app_handle())
            {
                log::error!("cannot open settings: {e}");
            }
        })
        .on_menu_event(|app, event| match event.id.as_ref() {
            "settings" => {
                if let Err(e) = crate::settings::open(app) {
                    log::error!("cannot open settings: {e}");
                }
            }
            "toggle" => {
                let visible = app
                    .get_webview_window(NOTCH_LABEL)
                    .and_then(|w| w.is_visible().ok())
                    .unwrap_or(false);
                crate::notch::set_user_visible(app, !visible);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
