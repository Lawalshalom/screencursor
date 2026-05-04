// System tray icon and menu (Tauri v2 TrayIconBuilder API).
// Handles start/stop toggling, settings window show, and quit. Gracefully
// handles missing tray icon asset.

use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Runtime,
};

pub fn create_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let menu = Menu::new(app)?;

    let start_stop_item = MenuItem::with_id(
        app,
        "start_stop",
        "Start Tracking",
        true,
        None::<&str>,
    )?;

    let settings_item = MenuItem::with_id(
        app,
        "settings",
        "Show Settings",
        true,
        None::<&str>,
    )?;

    let quit_item = MenuItem::with_id(
        app,
        "quit",
        "Quit",
        true,
        None::<&str>,
    )?;

    menu.append(&start_stop_item)?;
    menu.append(&settings_item)?;
    menu.append(&quit_item)?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                "start_stop" => {
                    let state: tauri::State<crate::AppState> = app.state();
                    let app_state = state.inner().clone();
                    let _ = crate::toggle_tracking(&app_state);
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "quit" => {
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let visible = window.is_visible().unwrap_or(false);
                    if visible {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
        });

    // Try to load tray icon, but don't fail if it doesn't exist
    if let Ok(resource_dir) = app.path().resource_dir() {
        let icon_path = resource_dir.join("icons/tray-icon.png");
        if icon_path.exists() {
            if let Ok(icon) = tauri::image::Image::from_path(icon_path) {
                builder = builder.icon(icon);
            }
        }
    }

    builder.build(app)?;

    Ok(())
}
