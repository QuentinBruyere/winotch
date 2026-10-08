//! Right click on a module in the notch (open part, closed notch, pin): a
//! native menu to pin or unpin it (DF-0012). Native, so it may go past the
//! notch window's visible shape.

use tauri::menu::{Menu, MenuEvent, MenuItem};
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::AppState;
use crate::config::PinSide;

const PIN: &str = "module-pin:";
const UNPIN: &str = "module-unpin:";

/// Shows the menu of `module` at the cursor.
#[tauri::command]
pub fn module_menu(window: WebviewWindow, module: String) -> Result<(), String> {
    let app = window.app_handle();
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    let Some(entry) = state.modules.iter().find(|m| m.id() == module) else {
        return Ok(());
    };
    let item = if config.pins.contains_key(&module) {
        MenuItem::with_id(
            app,
            format!("{UNPIN}{module}"),
            crate::t!("notch.menu.unpin"),
            true,
            None::<&str>,
        )
    } else {
        // The first enabled module is the notch itself: it cannot be pinned.
        let pinnable = crate::module::first_enabled(&state.modules, &config)
            .is_some_and(|first| first.id() != entry.id());
        MenuItem::with_id(
            app,
            format!("{PIN}{module}"),
            crate::t!("notch.menu.pin"),
            pinnable,
            None::<&str>,
        )
    }
    .map_err(|e| e.to_string())?;
    let menu = Menu::with_items(app, &[&item]).map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())
}

/// The choice made in the menu, from the app-wide menu events.
pub fn handle(app: &AppHandle, event: &MenuEvent) {
    let id = event.id().as_ref();
    if let Some(module) = id.strip_prefix(PIN) {
        crate::settings::set_module_pin(app.clone(), module.to_owned(), Some(PinSide::Auto));
    } else if let Some(module) = id.strip_prefix(UNPIN) {
        crate::settings::set_module_pin(app.clone(), module.to_owned(), None);
    }
}
