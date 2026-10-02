use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};

use crate::NOTCH_LABEL;

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
    let quit = MenuItem::with_id(app, "quit", "Quitter winotch", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&toggle, &connect, &disconnect, &separator, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(
            app.default_window_icon()
                .cloned()
                .expect("bundle icon is configured"),
        )
        .tooltip("winotch")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => {
                if let Some(w) = app.get_webview_window(NOTCH_LABEL) {
                    let visible = w.is_visible().unwrap_or(false);
                    let _ = if visible { w.hide() } else { w.show() };
                }
            }
            "connect" => crate::connect_claude_code(app, true),
            "disconnect" => crate::connect_claude_code(app, false),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
