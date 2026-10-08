//! Right click on a module in the notch (open part, closed notch, pin): a
//! native menu to pin or unpin it (DF-0012), after the clicked item's own
//! choices (`Module::item_menu`, e.g. a session's companion, DF-0020).
//! Native, so it may go past the notch window's visible shape.

use tauri::menu::{
    CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use tauri::{AppHandle, Manager, WebviewWindow};

use crate::AppState;
use crate::config::PinSide;

const PIN: &str = "module-pin:";
const UNPIN: &str = "module-unpin:";
/// `item-choice:<module>\t<item>\t<choice>`.
const ITEM_CHOICE: &str = "item-choice:";

/// Shows the menu of `module` at the cursor; `item` is the right-clicked
/// item of the module (`<module id>:<item id>`), if any.
#[tauri::command]
pub fn module_menu(
    window: WebviewWindow,
    module: String,
    item: Option<String>,
) -> Result<(), String> {
    let app = window.app_handle();
    let state = app.state::<AppState>();
    let config = state.config.lock().unwrap().clone();
    let Some(entry) = state.modules.iter().find(|m| m.id() == module) else {
        return Ok(());
    };
    let pin = if config.pins.contains_key(&module) {
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
    // The front names items `<module id>:<item id>`.
    let submenus = match item
        .as_deref()
        .and_then(|item| item.split_once(':'))
        .filter(|(owner, _)| *owner == module)
    {
        Some((_, item)) => entry
            .item_menu(item)
            .into_iter()
            .map(|menu| item_submenu(app, &module, item, menu))
            .collect::<Result<Vec<_>, _>>()?,
        None => Vec::new(),
    };
    let separator = PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?;
    let mut entries: Vec<&dyn IsMenuItem<tauri::Wry>> = Vec::new();
    for submenu in &submenus {
        entries.push(submenu);
    }
    if !submenus.is_empty() {
        entries.push(&separator);
    }
    entries.push(&pin);
    let menu = Menu::with_items(app, &entries).map_err(|e| e.to_string())?;
    window.popup_menu(&menu).map_err(|e| e.to_string())
}

/// An item's own choices, as a submenu of ticked entries.
fn item_submenu(
    app: &AppHandle,
    module: &str,
    item: &str,
    menu: crate::module::ItemMenu,
) -> Result<Submenu<tauri::Wry>, String> {
    let submenu = Submenu::new(app, menu.title, true).map_err(|e| e.to_string())?;
    for choice in menu.choices {
        let id = format!("{ITEM_CHOICE}{module}\t{item}\t{}", choice.id);
        let entry =
            CheckMenuItem::with_id(app, id, choice.label, true, choice.checked, None::<&str>)
                .map_err(|e| e.to_string())?;
        submenu.append(&entry).map_err(|e| e.to_string())?;
    }
    Ok(submenu)
}

/// The choice made in the menu, from the app-wide menu events.
pub fn handle(app: &AppHandle, event: &MenuEvent) {
    let id = event.id().as_ref();
    if let Some(module) = id.strip_prefix(PIN) {
        crate::settings::set_module_pin(app.clone(), module.to_owned(), Some(PinSide::Auto));
    } else if let Some(module) = id.strip_prefix(UNPIN) {
        crate::settings::set_module_pin(app.clone(), module.to_owned(), None);
    } else if let Some(choice) = id.strip_prefix(ITEM_CHOICE) {
        let mut parts = choice.splitn(3, '\t');
        let (Some(module), Some(item), Some(choice)) = (parts.next(), parts.next(), parts.next())
        else {
            return;
        };
        let state = app.state::<AppState>();
        if let Some(entry) = state.modules.iter().find(|m| m.id() == module) {
            entry.item_action(item, choice);
        }
    }
}
