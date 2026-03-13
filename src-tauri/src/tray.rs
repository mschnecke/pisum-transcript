use once_cell::sync::Lazy;
use std::sync::RwLock;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

static APP_HANDLE: Lazy<RwLock<Option<AppHandle>>> = Lazy::new(|| RwLock::new(None));

/// Set up the system tray icon and menu.
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Store the app handle globally for notifications from any module
    {
        let mut handle = APP_HANDLE.write().unwrap();
        *handle = Some(app.handle().clone());
    }

    let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&settings_item)
        .separator()
        .item(&quit_item)
        .build()?;

    let tray_icon = load_tray_icon(app);

    let _tray = TrayIconBuilder::new()
        .icon(tray_icon)
        .tooltip("Pisum Langue")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
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
        })
        .build(app)?;

    // Intercept window close to hide instead of quit
    if let Some(window) = app.get_webview_window("main") {
        let window_clone = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window_clone.hide();
            }
        });
    }

    tracing::info!("System tray initialized");
    Ok(())
}

/// Send an OS-native notification.
pub fn send_notification(title: &str, message: &str) {
    let handle = APP_HANDLE.read().unwrap();
    if let Some(app) = handle.as_ref() {
        use tauri_plugin_notification::NotificationExt;
        let _ = app
            .notification()
            .builder()
            .title(title)
            .body(message)
            .show();
    } else {
        tracing::warn!(
            "Cannot send notification (no app handle): {} - {}",
            title,
            message
        );
    }
}

/// Update the tray tooltip to show the active preset name.
pub fn set_tray_tooltip(preset_name: &str) {
    let handle = APP_HANDLE.read().unwrap();
    if let Some(app) = handle.as_ref() {
        if let Some(tray) = app.tray_by_id("main") {
            let tooltip = format!("Pisum Langue — {}", preset_name);
            let _ = tray.set_tooltip(Some(&tooltip));
        }
    }
}

/// Update the tray icon to reflect recording state.
/// This will be wired up in Phase 2 when audio recording is implemented.
pub fn set_recording_state(_recording: bool) {
    // TODO: Phase 2 - swap tray icon between idle and recording states
    tracing::debug!("Recording state changed: {}", _recording);
}

/// Load the tray icon image.
fn load_tray_icon(app: &tauri::App) -> Image<'static> {
    // Try to load from bundled resources first, then fall back to dev path
    let icon_paths = get_icon_search_paths(app);

    for path in &icon_paths {
        if path.exists() {
            match Image::from_path(path) {
                Ok(img) => {
                    tracing::debug!("Loaded tray icon from: {}", path.display());
                    return img;
                }
                Err(e) => {
                    tracing::warn!("Failed to load tray icon from {}: {}", path.display(), e);
                }
            }
        }
    }

    tracing::warn!("No tray icon found, using empty icon");
    Image::from_bytes(include_bytes!("../icons/tray-icon.png"))
        .expect("Failed to load embedded tray icon")
}

/// Get a list of possible tray icon paths to search.
fn get_icon_search_paths(app: &tauri::App) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    let icon_name = get_theme_icon_name();

    // Resource directory (production)
    if let Ok(resource_dir) = app.path().resource_dir() {
        paths.push(resource_dir.join(&icon_name));
        paths.push(resource_dir.join("icons").join(&icon_name));
    }

    // Dev mode: walk up from executable to find src-tauri/icons
    if let Ok(exe_path) = std::env::current_exe() {
        let mut dir = exe_path.parent().map(|p| p.to_path_buf());
        for _ in 0..5 {
            if let Some(ref d) = dir {
                let candidate = d.join("src-tauri").join("icons").join(&icon_name);
                paths.push(candidate);
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
    }

    paths
}

/// Get the appropriate icon filename based on system theme.
fn get_theme_icon_name() -> String {
    if is_dark_mode() {
        "tray-icon-light.png".to_string()
    } else {
        "tray-icon-dark.png".to_string()
    }
}

/// Detect if the system is in dark mode.
#[cfg(target_os = "windows")]
fn is_dark_mode() -> bool {
    use windows::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD,
    };
    use windows::core::w;

    let mut data: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;

    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut data as *mut u32 as *mut _),
            Some(&mut size),
        )
    };

    result.is_ok() && data == 0
}

#[cfg(target_os = "macos")]
fn is_dark_mode() -> bool {
    // macOS: template icons auto-adapt, but we detect for non-template use
    false
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn is_dark_mode() -> bool {
    false
}
