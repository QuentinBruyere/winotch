use tauri::{
    AppHandle, Manager,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_autostart::ManagerExt;

use crate::{AppState, NOTCH_LABEL};

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(
        app,
        "toggle",
        "Afficher / masquer le notch",
        true,
        None::<&str>,
    )?;
    let connect = MenuItem::with_id(app, "connect", "Connecter Claude Code", true, None::<&str>)?;
    let disconnect = MenuItem::with_id(
        app,
        "disconnect",
        "Déconnecter Claude Code",
        true,
        None::<&str>,
    )?;
    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);
    let autostart = CheckMenuItem::with_id(
        app,
        "autostart",
        "Lancer au démarrage de l'ordinateur",
        true,
        autostart_enabled,
        None::<&str>,
    )?;
    let sound_enabled = app.state::<AppState>().config.lock().unwrap().sound_enabled;
    let sound = CheckMenuItem::with_id(
        app,
        "sound",
        "Son des notifications",
        true,
        sound_enabled,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quitter winotch", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &toggle,
            &PredefinedMenuItem::separator(app)?,
            &connect,
            &disconnect,
            &PredefinedMenuItem::separator(app)?,
            &sound,
            &autostart,
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
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "toggle" => {
                let visible = app
                    .get_webview_window(NOTCH_LABEL)
                    .and_then(|w| w.is_visible().ok())
                    .unwrap_or(false);
                crate::notch::set_user_visible(app, !visible);
            }
            "connect" => crate::connect_claude_code(app, true),
            "disconnect" => crate::connect_claude_code(app, false),
            "autostart" => {
                let launcher = app.autolaunch();
                let enable = !launcher.is_enabled().unwrap_or(false);
                let result = if enable {
                    launcher.enable()
                } else {
                    launcher.disable()
                };
                if let Err(e) = result {
                    log::error!("cannot change autostart: {e}");
                }
                // Reflect the real state, whatever happened.
                let _ = autostart.set_checked(launcher.is_enabled().unwrap_or(false));
            }
            "sound" => {
                let enabled = !app.state::<AppState>().config.lock().unwrap().sound_enabled;
                crate::set_sound_enabled(app, enabled);
                let _ = sound.set_checked(enabled);
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
