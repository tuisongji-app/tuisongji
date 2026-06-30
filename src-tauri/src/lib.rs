mod bilibili_api;
mod poller;
mod state;

use state::{AppState, LiveStatus, Subscription, SubscriptionStatus};
use std::sync::Arc;
use log::{error, info, warn};
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_notification::NotificationExt;

// ---- Tauri Commands ----

#[tauri::command]
async fn add_subscription(
    uid: u64,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SubscriptionStatus, String> {
    // Check for duplicates
    {
        let subs = state.subscriptions.lock().unwrap();
        if subs.iter().any(|s| s.uid == uid) {
            return Err("该UID已添加".to_string());
        }
    }

    // Step 1: get user profile + room_id
    let (name, remote_avatar_url, room_id) = bilibili_api::get_master_info(uid).await?;

    // Step 2: download avatar to local cache
    let data_dir = state.data_dir.clone();
    if let Some(ref url) = remote_avatar_url {
        if let Err(e) = bilibili_api::download_avatar(url, uid, &data_dir).await {
            warn!("Avatar download failed: {}", e);
        }
    }

    // Step 3: get live status from room info
    let room_info = bilibili_api::get_room_info(room_id).await?;
    let status = LiveStatus::from_i64(room_info.live_status);

    // Add subscription
    {
        let mut subs = state.subscriptions.lock().unwrap();
        subs.push(Subscription {
            uid,
            name: Some(name.clone()),
            room_id: Some(room_id),
        });
    }

    // Initialize status cache
    {
        let mut cache = state.status_cache.lock().unwrap();
        cache.insert(uid, status.clone());
    }

    // Persist
    state.save();

    let result = SubscriptionStatus {
        uid,
        name,
        status,
        title: Some(room_info.title),
        room_id: Some(room_id),
        avatar_url: Some(state.avatar_full_path(uid)),
    };

    let _ = app_handle.emit("status-changed", &result);

    Ok(result)
}

#[tauri::command]
async fn remove_subscription(
    uid: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    {
        let mut subs = state.subscriptions.lock().unwrap();
        subs.retain(|s| s.uid != uid);
    }
    {
        let mut cache = state.status_cache.lock().unwrap();
        cache.remove(&uid);
    }
    state.save();
    Ok(())
}

#[tauri::command]
async fn list_subscriptions(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<SubscriptionStatus>, String> {
    let (subs, cache) = {
        let subs = state.subscriptions.lock().unwrap().clone();
        let cache = state.status_cache.lock().unwrap().clone();
        (subs, cache)
    };

    let result = subs
        .into_iter()
        .map(|s| {
            let status = cache.get(&s.uid).cloned().unwrap_or(LiveStatus::Offline);
            SubscriptionStatus {
                uid: s.uid,
                name: s.name.unwrap_or_else(|| "未知".to_string()),
                status,
                title: None,
                room_id: s.room_id,
                avatar_url: Some(state.avatar_full_path(s.uid)),
            }
        })
        .collect();

    Ok(result)
}

#[tauri::command]
async fn refresh_status(
    uid: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SubscriptionStatus, String> {
    // Get room_id from cached subscription
    let room_id = {
        let subs = state.subscriptions.lock().unwrap();
        subs.iter()
            .find(|s| s.uid == uid)
            .and_then(|s| s.room_id)
            .ok_or("未找到该订阅".to_string())?
    };

    let room_info = bilibili_api::get_room_info(room_id).await?;
    let status = LiveStatus::from_i64(room_info.live_status);

    {
        let mut cache = state.status_cache.lock().unwrap();
        cache.insert(uid, status.clone());
    }

    let sub_name = {
        let subs = state.subscriptions.lock().unwrap();
        subs.iter()
            .find(|s| s.uid == uid)
            .and_then(|s| s.name.clone())
            .unwrap_or_else(|| "未知".to_string())
    };

    Ok(SubscriptionStatus {
        uid,
        name: sub_name,
        status,
        title: Some(room_info.title),
        room_id: Some(room_id),
        avatar_url: Some(state.avatar_full_path(uid)),
    })
}

#[tauri::command]
async fn update_poll_interval(
    interval_secs: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let min_interval = 10;
    if interval_secs < min_interval {
        return Err(format!("轮询间隔不能小于{}秒", min_interval));
    }
    {
        let mut config = state.config.lock().unwrap();
        config.poll_interval_secs = interval_secs;
    }
    state.save();
    Ok(())
}

#[tauri::command]
async fn get_config(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<state::AppConfig, String> {
    let config = state.config.lock().unwrap().clone();
    Ok(config)
}

// Shared notification — used by poller (via poller.rs) and test command
pub fn notify_status_change(
    app_handle: &tauri::AppHandle,
    name: &str,
    prev_status: &LiveStatus,
    new_status: &LiveStatus,
    live_title: Option<&str>,
) {
    let (title, body) = match (prev_status, new_status) {
        (LiveStatus::Offline, LiveStatus::Live) => {
            let b = live_title.unwrap_or("正在直播");
            (format!("推送姬 - {} 开播了!", name), b.to_string())
        }
        (LiveStatus::Live, LiveStatus::Offline) => {
            (format!("推送姬 - {} 已结束直播", name), "直播已结束".to_string())
        }
        _ => return,
    };

    match app_handle.notification().builder().title(title).body(body).show() {
        Ok(_) => info!("Notification sent: {} {:?}→{:?}", name, prev_status, new_status),
        Err(e) => error!("Notification failed: {:?}", e),
    }
}

#[tauri::command]
async fn test_trigger_status(
    uid: u64,
    target_status: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let new_status = match target_status.as_str() {
        "live" => LiveStatus::Live,
        "offline" => LiveStatus::Offline,
        "replay" => LiveStatus::Replay,
        _ => return Err(format!("无效状态: {}", target_status)),
    };

    let prev_status = {
        let mut cache = state.status_cache.lock().unwrap();
        cache.insert(uid, new_status.clone()).unwrap_or(LiveStatus::Offline)
    };

    let (name, room_id) = {
        let subs = state.subscriptions.lock().unwrap();
        let sub = subs.iter().find(|s| s.uid == uid).ok_or("订阅不存在")?;
        (
            sub.name.clone().unwrap_or_else(|| "未知".to_string()),
            sub.room_id,
        )
    };

    let status_update = SubscriptionStatus {
        uid,
        name: name.clone(),
        status: new_status.clone(),
        title: if new_status == LiveStatus::Live { Some("【测试】模拟开播标题".to_string()) } else { None },
        room_id,
        avatar_url: Some(state.avatar_full_path(uid)),
    };
    let _ = app_handle.emit("status-changed", &status_update);

    // Delay so user can switch focus away from app to see notification
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    notify_status_change(&app_handle, &name, &prev_status, &new_status, None);

    Ok(())
}

// ---- App Entry Point ----

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .setup(|app| {

            // Determine data directory
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));

            let state = Arc::new(AppState::new(data_dir));

            // Start background poller
            poller::start_poller(app.handle().clone(), state.clone());

            // Manage state
            app.manage(state);

            // Build tray menu
            let show = tauri::menu::MenuItemBuilder::with_id("show", "显示设置")
                .build(app)?;
            let quit = tauri::menu::MenuItemBuilder::with_id("quit", "退出")
                .build(app)?;
            let menu = tauri::menu::MenuBuilder::new(app)
                .item(&show)
                .item(&quit)
                .build()?;

            // Build tray icon
            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .expect("Failed to load tray icon");

            let _tray = tauri::tray::TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("推送姬")
                .on_menu_event(|app_handle, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app_handle.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray_icon, event| {
                    if let tauri::tray::TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray_icon.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            add_subscription,
            remove_subscription,
            list_subscriptions,
            refresh_status,
            update_poll_interval,
            get_config,
            test_trigger_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
