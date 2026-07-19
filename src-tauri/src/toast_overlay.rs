//! Toast-style notification overlay window.

use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Listener, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct ToastItem {
    pub room_id: u64,
    pub name: String,
    pub action: String,
    pub avatar_path: Option<String>,
    pub sub_type: String,
}

// ---------------------------------------------------------------------------
// Layout constants (logical pixels)
// ---------------------------------------------------------------------------

const WIN_W: f64 = 320.0;
const CARD_VISIBLE: f64 = 36.0;
const OVERLAP: f64 = 28.0;
const HEADER_H: f64 = 32.0;
const PAD: f64 = 8.0;
const EDGE: f64 = 16.0;

/// Set to true when the overlay frontend signals it has mounted and is
/// ready to render.  Until then `update_overlay_state` emits events but
/// does not show the window — avoids WebView2 white flash on first show.
static READY: AtomicBool = AtomicBool::new(false);

// ---------------------------------------------------------------------------
// Windows work-area helper (excludes taskbar)
// ---------------------------------------------------------------------------

fn work_area(app_handle: &AppHandle) -> (f64, f64, f64, f64) {
    let scale = app_handle
        .primary_monitor()
        .ok()
        .flatten()
        .map(|m| m.scale_factor())
        .unwrap_or(1.0);
    if scale <= 0.0 {
        return monitor_size(app_handle);
    }

    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        };
        use windows::Win32::Foundation::RECT;

        let mut rect = RECT::default();
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut rect as *mut RECT as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };
        if ok.is_ok() {
            return (
                rect.left as f64 / scale,
                rect.top as f64 / scale,
                rect.right as f64 / scale,
                rect.bottom as f64 / scale,
            );
        }
    }

    monitor_size(app_handle)
}

fn monitor_size(app_handle: &AppHandle) -> (f64, f64, f64, f64) {
    if let Ok(Some(monitor)) = app_handle.primary_monitor() {
        let ms = monitor.size();
        let scale = monitor.scale_factor();
        return (0.0, 0.0, ms.width as f64 / scale, ms.height as f64 / scale);
    }
    (0.0, 0.0, 1920.0, 1080.0)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn show_at(window: &tauri::WebviewWindow, win_h: f64) {
    let (_left, _top, work_w, work_h) = work_area(window.app_handle());
    let x = work_w - WIN_W - EDGE;
    let y = work_h - win_h - EDGE;
    let _ = window.set_position(LogicalPosition::new(x, y));
    let _ = window.set_size(LogicalSize::new(WIN_W, win_h));
    let _ = window.show();
}

/// Collapsed window height.
const COLLAPSED_H: f64 = 64.0;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Shrink the overlay to collapsed size and reposition to bottom-right.
pub fn collapse(app_handle: &AppHandle) {
    if let Some(window) = app_handle.get_webview_window("toast-overlay") {
        let (_left, _top, work_w, work_h) = work_area(app_handle);
        let x = work_w - WIN_W - EDGE;
        let y = work_h - COLLAPSED_H - EDGE;
        let _ = window.set_size(LogicalSize::new(WIN_W, COLLAPSED_H));
        let _ = window.set_position(LogicalPosition::new(x, y));
    }
}

pub fn create_overlay_window(app_handle: &AppHandle) {
    if app_handle.get_webview_window("toast-overlay").is_some() {
        return;
    }

    let window = WebviewWindowBuilder::new(app_handle, "toast-overlay", WebviewUrl::default())
        .title("")
        .inner_size(WIN_W, 200.0)
        .min_inner_size(WIN_W, 0.0)
        .max_inner_size(WIN_W, 520.0)
        .decorations(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .resizable(false)
        .transparent(true)
        .shadow(false)
        .build()
        .expect("Failed to create toast overlay window");

    // When the frontend Vue app mounts it emits "overlay-ready".  Until
    // then we only emit events — never call show() — so WebView2's first
    // paint happens off-screen during page load, not when the user sees it.
    let h = app_handle.clone();
    window.listen("overlay-ready", move |_| {
        READY.store(true, Ordering::SeqCst);
        let count = crate::NOTIFS.lock().unwrap().len();
        if count > 0 {
            update_overlay_state(&h);
        }
    });

    // Expand from collapsed state when the user clicks the indicator.
    let h2 = app_handle.clone();
    window.listen("toast-expand", move |_| {
        update_overlay_state(&h2);
    });
}

pub fn update_overlay_state(app_handle: &AppHandle) {
    let items: Vec<ToastItem> = {
        let notifs = crate::NOTIFS.lock().unwrap();
        notifs
            .iter()
            .map(|n| ToastItem {
                room_id: n.room_id,
                name: n.name.clone(),
                action: n.action.clone(),
                avatar_path: n.avatar_path.clone(),
                sub_type: n.sub_type.clone(),
            })
            .collect()
    };

    let _ = app_handle.emit("toast-state", &items);

    let ready = READY.load(Ordering::SeqCst);

    if let Some(window) = app_handle.get_webview_window("toast-overlay") {
        if items.is_empty() {
            if ready {
                let _ = window.hide();
            }
        } else if ready {
            let n = items.len() as f64;
            let body_h = CARD_VISIBLE * n + OVERLAP;
            let win_h = HEADER_H + body_h + PAD * 2.0;
            show_at(&window, win_h);
        }
    }
}
