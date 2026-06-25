mod api;
mod auth;
mod commands;

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(target_os = "windows")]
use std::sync::Mutex;

use tauri::{
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow,
};

const TRAY_ID: &str = "main-tray";
#[cfg(target_os = "macos")]
const HIDE_DELAY: Duration = Duration::from_secs(3);
#[cfg(not(target_os = "macos"))]
const HIDE_DELAY: Duration = Duration::from_millis(500);
/// Popup width in logical pixels. Must match the `width` in tauri.conf.json
/// and the value used by `fit_height`, otherwise the window visibly resizes
/// each time the tray is clicked.
const POPUP_WIDTH: f64 = 340.0;

/// Flag to suppress auto-hide when the frontend intentionally keeps the window open
/// (e.g. while the login form is active).
static ALLOW_AUTO_HIDE: AtomicBool = AtomicBool::new(true);
/// Set on focus-loss, cleared on focus-gain — prevents a delayed hide if the
/// window regains focus before the timer fires.
static PENDING_HIDE: AtomicBool = AtomicBool::new(false);
/// Mouse is hovering over the tray icon — cancel auto-hide.
static MOUSE_ON_TRAY: AtomicBool = AtomicBool::new(false);
/// Mouse is hovering inside the popup window — cancel auto-hide.
static MOUSE_IN_WINDOW: AtomicBool = AtomicBool::new(false);

/// Start a deferred auto-hide timer.  The window hides only if, after
/// `HIDE_DELAY`, all of these hold: PENDING_HIDE is still true,
/// ALLOW_AUTO_HIDE is true, and neither tray nor popup is hovered.
/// Safe to call multiple times — only the last timer's verdict matters.
fn start_hide_timer(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        if !win.is_visible().unwrap_or(false) {
            return;
        }
        PENDING_HIDE.store(true, Ordering::SeqCst);
        let win = win.clone();
        std::thread::spawn(move || {
            std::thread::sleep(HIDE_DELAY);
            if PENDING_HIDE.load(Ordering::SeqCst)
                && ALLOW_AUTO_HIDE.load(Ordering::SeqCst)
                && !MOUSE_ON_TRAY.load(Ordering::SeqCst)
                && !MOUSE_IN_WINDOW.load(Ordering::SeqCst)
            {
                let _ = win.hide();
            }
        });
    }
}

/// Y-coordinate anchor (logical pixels) of the click that opened the window.
/// Used on Windows to keep the window's bottom edge at the correct position
/// when `fit_height` adjusts the height.
#[cfg(target_os = "windows")]
static ANCHOR_Y: Mutex<Option<f64>> = Mutex::new(None);

/// Show the popup window anchored just below the tray icon on macOS.
/// The webview is 340 wide; center it under the click x.
#[cfg(target_os = "macos")]
fn show_popup_at(window: &WebviewWindow, x: f64, y: f64) {
    let w = POPUP_WIDTH;
    // Reuse the current height if the frontend has already fitted it to the
    // content; otherwise fall back to the default. We avoid forcing 510 every
    // time, which would override the height the frontend measured.
    let h = window
        .outer_size()
        .ok()
        .map(|s| s.to_logical(window.scale_factor().unwrap_or(1.0)).height)
        .filter(|h| *h > 1.0)
        .unwrap_or(510.0);
    let _ = window.set_position(tauri::LogicalPosition::new(x - w / 2.0, y));
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
    let _ = window.show();
    let _ = window.set_focus();
}

/// Show the popup window anchored just above the tray area on Windows.
/// Windows taskbar is typically at the bottom, so the window pops upward.
#[cfg(target_os = "windows")]
fn show_popup_at(window: &WebviewWindow, x: f64, y: f64) {
    let sf = window.scale_factor().unwrap_or(1.0);
    let w = POPUP_WIDTH;
    // Reuse the current fitted height; fall back to 510.
    let h = window
        .outer_size()
        .ok()
        .map(|s| s.to_logical(sf).height)
        .filter(|h| *h > 1.0)
        .unwrap_or(510.0);
    // Center horizontally on the click; position the window so its bottom
    // edge sits 8 px above the click point (near the system tray).
    let pos_x = (x - w / 2.0).max(0.0);
    let pos_y = (y - h - 8.0).max(0.0);
    let _ = window.set_position(tauri::LogicalPosition::new(pos_x, pos_y));
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
    let _ = window.show();
    let _ = window.set_focus();

    // Remember the click Y so fit_height can re-anchor the window bottom.
    if let Ok(mut anchor) = ANCHOR_Y.lock() {
        *anchor = Some(y);
    }
}

/// Fallback for Linux / other platforms.
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn show_popup_at(window: &WebviewWindow, x: f64, y: f64) {
    let w = POPUP_WIDTH;
    let h = window
        .outer_size()
        .ok()
        .map(|s| s.to_logical(window.scale_factor().unwrap_or(1.0)).height)
        .filter(|h| *h > 1.0)
        .unwrap_or(510.0);
    let _ = window.set_position(tauri::LogicalPosition::new(x - w / 2.0, y));
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
    let _ = window.show();
    let _ = window.set_focus();
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    // Do NOT attach a menu to the tray icon — on macOS a menu intercepts
    // left-click and prevents on_tray_icon_event from firing, which means
    // the popup window can never appear.  On Windows the popup is driven
    // by click events as well.  Quit is handled via the UI instead.
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().cloned().expect("missing icon"))
        .tooltip("TokenHub Monitor")
        .on_tray_icon_event(|tray, event| {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    position,
                    ..
                } => {
                    let app = tray.app_handle();
                    if let Some(win) = app.get_webview_window("main") {
                        if win.is_visible().unwrap_or(false) {
                            let _ = win.hide();
                        } else {
                            let sf = win.scale_factor().unwrap_or(1.0);
                            let lx = position.x / sf;
                            let ly = position.y / sf;
                            show_popup_at(&win, lx, ly);
                        }
                    }
                }
                TrayIconEvent::Enter { .. } => {
                    // Mouse over tray → keep window visible.
                    MOUSE_ON_TRAY.store(true, Ordering::SeqCst);
                    PENDING_HIDE.store(false, Ordering::SeqCst);
                }
                TrayIconEvent::Leave { .. } => {
                    MOUSE_ON_TRAY.store(false, Ordering::SeqCst);
                    // Window visible + mouse away from both tray and popup → start timer.
                    // Focus events are unreliable on Windows for this window style,
                    // so we rely on mouse-position signals instead.
                    start_hide_timer(&tray.app_handle());
                }
                _ => {}
            }
        });

    // Start with a placeholder title; the frontend updates it.
    builder = builder.title("");

    let _tray = builder.build(app)?;
    Ok(())
}

/// Allow the frontend to control whether the window auto-hides on focus loss.
/// The login form needs the window to stay open even when focus briefly leaves.
#[tauri::command]
fn set_auto_hide(enabled: bool) {
    ALLOW_AUTO_HIDE.store(enabled, Ordering::SeqCst);
}

/// Let the backend know whether the mouse cursor is inside the popup window.
/// The frontend fires this via mouseenter / mouseleave on the app root.
#[tauri::command]
fn set_mouse_in_window(app: tauri::AppHandle, in_window: bool) {
    MOUSE_IN_WINDOW.store(in_window, Ordering::SeqCst);
    if in_window {
        PENDING_HIDE.store(false, Ordering::SeqCst);
    } else if ALLOW_AUTO_HIDE.load(Ordering::SeqCst) {
        start_hide_timer(&app);
    }
}

/// Frontend-facing command so the webview can trigger a hide timer
/// (e.g. from a window `blur` event that the Rust WindowEvent layer
/// doesn't reliably emit on Windows).
#[tauri::command]
fn start_hide_timer_cmd(app: tauri::AppHandle) {
    if ALLOW_AUTO_HIDE.load(Ordering::SeqCst) {
        start_hide_timer(&app);
    }
}

/// Quit the application from the UI (no tray menu).
#[tauri::command]
fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

/// Update the tray title (shown next to the icon in the menu bar on macOS;
/// no‑op on Windows where the tray only displays the icon).
#[tauri::command]
fn set_tray_title(app: tauri::AppHandle, title: String) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_title(Some(&title));
    }
}

/// Resize the popup height to match the rendered content. Width stays fixed.
/// `height` is in logical (CSS) pixels as measured by the frontend.
/// On Windows the window bottom is also re-anchored to prevent gaps or
/// overflow near the taskbar.
#[tauri::command]
fn fit_height(app: tauri::AppHandle, height: f64) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.set_size(tauri::LogicalSize::new(POPUP_WIDTH, height));

        #[cfg(target_os = "windows")]
        {
            if let Ok(anchor) = ANCHOR_Y.lock() {
                if let Some(ay) = *anchor {
                    let sf = win.scale_factor().unwrap_or(1.0);
                    if let Ok(pos) = win.outer_position() {
                        let current_x = pos.to_logical::<f64>(sf).x;
                        // Keep the window bottom at `ay - 8` (the anchor
                        // stored when show_popup_at placed the window).
                        let new_y = (ay - height - 8.0).max(0.0);
                        let _ = win.set_position(tauri::LogicalPosition::new(current_x, new_y));
                    }
                }
            }
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::save_master_key,
            commands::clear_master_key,
            commands::has_master_key,
            commands::fetch_package,
            set_auto_hide,
            set_mouse_in_window,
            start_hide_timer_cmd,
            quit_app,
            set_tray_title,
            fit_height,
        ])
        .setup(|app| {
            // macOS: Dock icon is hidden via LSUIElement=true in Info.plist.
            // Windows: skipTaskbar=true in tauri.conf.json achieves the same.
            build_tray(app)?;

            // macOS: keep the popup out of Cmd+Tab switcher, Mission Control,
            // and App Exposé by setting the window collection behavior to
            // Transient (2) | IgnoresCycle (4) = 6.
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                if let Ok(ns_window) = window.ns_window() {
                    use objc::{msg_send, sel, sel_impl};
                    unsafe {
                        let () = msg_send![
                            ns_window as *mut objc::runtime::Object,
                            setCollectionBehavior: 6u64
                        ];
                    }
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::Focused(false) => {
                // Focus events are unreliable on Windows for our window style.
                // This path still fires on macOS; Windows relies on mouse-position
                // and blur signals instead.
                start_hide_timer(window.app_handle());
            }
            tauri::WindowEvent::Focused(true) => {
                // Window came back — cancel any pending delayed hide.
                PENDING_HIDE.store(false, Ordering::SeqCst);
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
