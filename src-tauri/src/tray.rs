use tauri::{
    AppHandle, Manager,
    menu::{Menu, MenuItem},
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
    let quit = MenuItem::with_id(app, "quit", "Quitter winotch", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &quit])?;

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
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}
