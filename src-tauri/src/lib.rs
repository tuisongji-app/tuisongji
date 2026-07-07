mod api;
mod github_sounds;
mod poller;
mod sound;
mod state;
mod updater;

use api::PlatformApi;
use log::{info, warn};
use state::{AppState, LiveStatus, Subscription, SubscriptionStatus};
use std::sync::Mutex;
use tauri::{Context, Manager, Runtime, is_dev};
use std::sync::Arc;
use store::figment::value::Value;
use tauri::Emitter;
use tauri_plugin_autostart::AutoLaunchManager;
use tauri_plugin_opener::OpenerExt;

// ---- Notification queue ----

struct NotifItem {
    room_id: u64,
    name: String,
    action: String,
    avatar_path: Option<String>,
    sub_type: String,
}

static NOTIFS: Mutex<Vec<NotifItem>> = Mutex::new(Vec::new());
static BADGE_TIMER: Mutex<Option<tauri::async_runtime::JoinHandle<()>>> = Mutex::new(None);

fn reset_title_timer(app_handle: tauri::AppHandle) {
    let mut timer = BADGE_TIMER.lock().unwrap();
    if let Some(h) = timer.take() {
        h.abort();
    }

    let h = app_handle.clone();
    let timeout_mins = app_handle
        .try_state::<Arc<AppState>>()
        .map(|s| {
            let store = s.store.lock().unwrap();
            store.get_all().config.badge_timeout_mins
        })
        .unwrap_or(30);

    *timer = Some(tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(timeout_mins * 60)).await;
        if let Some(tray) = h.tray_by_id("main") {
            let notifs = NOTIFS.lock().unwrap();
            if !notifs.is_empty() {
                let _ = tray.set_title(Some(&format!("({})", notifs.len())));
            }
        }
    }));
}

fn load_avatar_icon(path: &str) -> Option<tauri::image::Image<'static>> {
    use image::imageops::FilterType;

    let bytes = std::fs::read(path).ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let src = img.to_rgba8();

    // Resize to crop square then round
    let min_dim = src.dimensions().0.min(src.dimensions().1);
    let cropped = image::imageops::crop_imm(&src, 0, 0, min_dim, min_dim).to_image();
    let size = 48u32; // larger tray icon
    let resized = image::imageops::resize(&cropped, size, size, FilterType::Lanczos3);

    let mut dst = image::RgbaImage::new(size, size);
    let cx = (size as f32) / 2.0;
    let r = cx;

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 - cx;
            let dy = y as f32 - cx;
            if dx * dx + dy * dy <= r * r {
                dst.put_pixel(x, y, *resized.get_pixel(x, y));
            }
        }
    }

    let raw = dst.into_raw();
    Some(tauri::image::Image::new_owned(raw, size, size))
}

fn create_window(app_handle: &tauri::AppHandle, show_on_startup: bool) {
    let builder = tauri::WebviewWindowBuilder::new(app_handle, "main", tauri::WebviewUrl::default())
        .title("推送姬")
        .inner_size(800.0, 600.0)
        .visible(false);
    let window = builder.build().unwrap();
    if show_on_startup {
        let _ = window.show();
    }
}

fn show_window(app_handle: &tauri::AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn rebuild_tray_menu(app_handle: &tauri::AppHandle) {
    let notifs = NOTIFS.lock().unwrap();
    let mut menu = tauri::menu::MenuBuilder::new(app_handle);

    // Group header + notification items
    if !notifs.is_empty() {
        let header_label = &notifs[0].sub_type;
        let header = tauri::menu::MenuItemBuilder::with_id("platform_header", header_label)
            .enabled(false)
            .build(app_handle)
            .unwrap();
        menu = menu.item(&header);
    }
    for item in notifs.iter() {
        let label = format!("{} {}", item.name, item.action);
        let id = format!("notif:{}", item.room_id);

        // Try to use IconMenuItemBuilder with avatar
        let mi: Box<dyn tauri::menu::IsMenuItem<tauri::Wry>> =
            if let Some(ref path) = item.avatar_path {
                if let Some(icon) = load_avatar_icon(path) {
                    match tauri::menu::IconMenuItemBuilder::with_id(
                        id.clone(),
                        label.clone(),
                    )
                    .icon(icon)
                    .build(app_handle)
                    {
                        Ok(icon_mi) => Box::new(icon_mi),
                        Err(_) => Box::new(
                            tauri::menu::MenuItemBuilder::with_id(id, label)
                                .build(app_handle)
                                .unwrap(),
                        ),
                    }
                } else {
                    Box::new(
                        tauri::menu::MenuItemBuilder::with_id(id, label)
                            .build(app_handle)
                            .unwrap(),
                    )
                }
            } else {
                Box::new(
                    tauri::menu::MenuItemBuilder::with_id(id, label)
                        .build(app_handle)
                        .unwrap(),
                )
            };
        menu = menu.item(&*mi);
    }

    // Separator + clear all
    if !notifs.is_empty() {
        let clear = tauri::menu::MenuItemBuilder::with_id("clear_all", "清空全部")
            .build(app_handle)
            .unwrap();
        menu = menu.separator().item(&clear);
    }

    // Separator + show + quit
    let show_item = tauri::menu::MenuItemBuilder::with_id("show", "显示界面")
        .build(app_handle)
        .unwrap();
    let quit_item = tauri::menu::MenuItemBuilder::with_id("quit", "退出")
        .build(app_handle)
        .unwrap();
    let menu = menu.separator().item(&show_item).item(&quit_item).build().unwrap();

    // Update tray
    if let Some(tray) = app_handle.tray_by_id("main") {
        if let Some(first) = notifs.first() {
            let _ =
                tray.set_title(Some(&format!("{} {}", first.name, first.action)));
        } else {
            let _ = tray.set_title(Some(""));
        }
        let _ = tray.set_menu(Some(menu));
    }
}

// ---- Tauri Commands ----

#[tauri::command]
async fn add_subscription(
    uid: u64,
    sub_type: String,
    input_mode: String,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SubscriptionStatus, String> {
    let (resolved_uid, name, remote_avatar_url, room_id) = match sub_type.as_str() {
        "bilibili" if input_mode == "room" => api::bilibili::BilibiliApi::get_master_info_by_room(uid).await?,
        "bilibili" => api::bilibili::BilibiliApi::get_master_info(uid).await?,
        "huya" if input_mode == "room" => api::huya::HuyaApi::get_master_info_by_room(uid).await?,
        "huya" => api::huya::HuyaApi::get_master_info(uid).await?,
        _ => return Err(format!("不支持的平台: {}", sub_type)),
    };

    {
        let store = state.store.lock().unwrap();
        let data = store.get_all();
        if data.subscriptions.iter().any(|s| s.uid == resolved_uid && s.r#type == sub_type) {
            return Err("该ID已添加".to_string());
        }
    }

    let data_dir = state.data_dir.clone();
    if let Some(ref url) = remote_avatar_url {
        if let Err(e) = api::download_avatar(url, resolved_uid, &sub_type, &data_dir).await {
            warn!("Avatar download failed: {}", e);
        }
    }

    let (live_status_code, title) = match sub_type.as_str() {
        "bilibili" => api::bilibili::BilibiliApi::get_room_info(room_id).await?,
        "huya" => api::huya::HuyaApi::get_room_info(room_id).await?,
        _ => return Err(format!("不支持的平台: {}", sub_type)),
    };
    let status = match sub_type.as_str() {
        "bilibili" => api::bilibili::BilibiliApi::map_live_status(live_status_code),
        "huya" => api::huya::HuyaApi::map_live_status(live_status_code),
        _ => return Err(format!("不支持的平台: {}", sub_type)),
    };

    {
        let mut store = state.store.lock().unwrap();
        let mut data = store.get_all();
        data.subscriptions.push(Subscription {
            uid: resolved_uid,
            name: Some(name.clone()),
            room_id: Some(room_id),
            r#type: sub_type.clone(),
        });
        let value =
            Value::serialize(&data.subscriptions).map_err(|e| format!("Serialize: {}", e))?;
        store
            .set("subscriptions", value)
            .map_err(|e| format!("Store: {}", e))?;
    }

    {
        let mut cache = state.status_cache.lock().unwrap();
        cache.insert((sub_type.clone(), resolved_uid), status.clone());
    }

    let result = SubscriptionStatus {
        uid: resolved_uid,
        sub_type: sub_type.clone(),
        name,
        status,
        title: Some(title),
        room_id: Some(room_id),
        avatar_url: Some(state.avatar_full_path(&sub_type, resolved_uid)),
    };

    let _ = app_handle.emit("status-changed", &result);
    Ok(result)
}

#[tauri::command]
async fn remove_subscription(
    uid: u64,
    sub_type: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    {
        let mut store = state.store.lock().unwrap();
        let mut data = store.get_all();
        data.subscriptions.retain(|s| !(s.uid == uid && s.r#type == sub_type));
        let value =
            Value::serialize(&data.subscriptions).map_err(|e| format!("Serialize: {}", e))?;
        store
            .set("subscriptions", value)
            .map_err(|e| format!("Store: {}", e))?;
    }
    {
        let mut cache = state.status_cache.lock().unwrap();
        cache.remove(&(sub_type, uid));
    }
    Ok(())
}

#[tauri::command]
async fn list_subscriptions(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<SubscriptionStatus>, String> {
    let (subs, cache) = {
        let store = state.store.lock().unwrap();
        let data = store.get_all();
        let cache = state.status_cache.lock().unwrap().clone();
        (data.subscriptions, cache)
    };

    let result = subs
        .into_iter()
        .map(|s| {
            let cache_key = (s.r#type.clone(), s.uid);
            let status = cache.get(&cache_key).cloned().unwrap_or(LiveStatus::Offline);
            SubscriptionStatus {
                uid: s.uid,
                sub_type: s.r#type.clone(),
                name: s.name.unwrap_or_else(|| "未知".to_string()),
                status,
                title: None,
                room_id: s.room_id,
                avatar_url: Some(state.avatar_full_path(&s.r#type, s.uid)),
            }
        })
        .collect();

    Ok(result)
}

#[tauri::command]
async fn refresh_status(
    uid: u64,
    sub_type: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<SubscriptionStatus, String> {
    let (room_id, sub_name) = {
        let store = state.store.lock().unwrap();
        let data = store.get_all();
        let sub = data
            .subscriptions
            .iter()
            .find(|s| s.uid == uid && s.r#type == sub_type)
            .ok_or("未找到该订阅".to_string())?;
        (sub.room_id.ok_or("该订阅缺少房间ID".to_string())?, sub.name.clone())
    };

    let (live_status_code, title) = match sub_type.as_str() {
        "bilibili" => api::bilibili::BilibiliApi::get_room_info(room_id).await?,
        "huya" => api::huya::HuyaApi::get_room_info(room_id).await?,
        _ => return Err(format!("不支持的平台: {}", sub_type)),
    };
    let status = match sub_type.as_str() {
        "bilibili" => api::bilibili::BilibiliApi::map_live_status(live_status_code),
        "huya" => api::huya::HuyaApi::map_live_status(live_status_code),
        _ => return Err(format!("不支持的平台: {}", sub_type)),
    };

    {
        let mut cache = state.status_cache.lock().unwrap();
        cache.insert((sub_type.clone(), uid), status.clone());
    }

    Ok(SubscriptionStatus {
        uid,
        sub_type: sub_type.clone(),
        name: sub_name.unwrap_or_else(|| "未知".to_string()),
        status,
        title: Some(title),
        room_id: Some(room_id),
        avatar_url: Some(state.avatar_full_path(&sub_type, uid)),
    })
}

#[tauri::command]
async fn update_poll_interval(
    interval_mins: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if interval_mins < 1 {
        return Err("轮询间隔不能小于1分钟".to_string());
    }
    let mut store = state.store.lock().unwrap();
    store
        .set("config.poll_interval_mins", interval_mins)
        .map_err(|e| format!("Store: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn update_badge_timeout(
    timeout_mins: u64,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if timeout_mins < 1 {
        return Err("超时时间不能小于1分钟".to_string());
    }
    let mut store = state.store.lock().unwrap();
    store
        .set("config.badge_timeout_mins", timeout_mins)
        .map_err(|e| format!("Store: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn get_config(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<state::AppConfig, String> {
    let store = state.store.lock().unwrap();
    Ok(store.get_all().config)
}

// ---- Sound commands ----

#[tauri::command]
async fn download_streamer_sounds(
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<state::SoundInfo, String> {
    let (avail_live, avail_offline) =
        github_sounds::available_sounds_for_name(&name).await.unwrap_or((0, 0));

    github_sounds::download_sounds_for_name(&name, &state.data_dir).await?;

    let (dl_live, dl_offline) = sound::count_downloaded(&name, &state.data_dir);

    Ok(state::SoundInfo {
        name,
        available_live: avail_live,
        available_offline: avail_offline,
        downloaded_live: dl_live,
        downloaded_offline: dl_offline,
    })
}

#[tauri::command]
async fn get_sound_info(
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<state::SoundInfo, String> {
    let (avail_live, avail_offline) =
        github_sounds::available_sounds_for_name(&name).await.unwrap_or((0, 0));
    let (dl_live, dl_offline) = sound::count_downloaded(&name, &state.data_dir);

    Ok(state::SoundInfo {
        name,
        available_live: avail_live,
        available_offline: avail_offline,
        downloaded_live: dl_live,
        downloaded_offline: dl_offline,
    })
}

#[tauri::command]
async fn play_streamer_sound(
    name: String,
    event_type: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let config = {
        let store = state.store.lock().unwrap();
        store.get_all().config
    };
    sound::play_random_for_streamer(&name, &event_type, config.sound_volume, &state.data_dir)
}

#[tauri::command]
async fn set_sound_enabled(
    enabled: bool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut store = state.store.lock().unwrap();
    store
        .set("config.sound_enabled", enabled)
        .map_err(|e| format!("Store: {}", e))?;
    Ok(())
}

#[tauri::command]
async fn set_sound_volume(
    volume: f32,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let clamped = volume.clamp(0.0, 1.0);
    let mut store = state.store.lock().unwrap();
    store
        .set("config.sound_volume", clamped)
        .map_err(|e| format!("Store: {}", e))?;
    Ok(())
}

pub fn notify_status_change(
    app_handle: &tauri::AppHandle,
    _uid: u64,
    sub_type: &str,
    name: &str,
    prev_status: &LiveStatus,
    new_status: &LiveStatus,
    room_id: Option<u64>,
    avatar_path: Option<&str>,
) {
    let was_live = *prev_status == LiveStatus::Live;
    let is_live = *new_status == LiveStatus::Live;

    let action = match (was_live, is_live) {
        (false, true) => "开播",
        (true, false) => "下播",
        _ => return,
    };

    let rid = room_id.unwrap_or(0);
    if rid == 0 {
        return;
    }

    // Push to front (newest first)
    {
        let mut notifs = NOTIFS.lock().unwrap();
        // Remove duplicate for same room_id if exists
        notifs.retain(|n| n.room_id != rid);
        notifs.insert(
            0,
            NotifItem {
                room_id: rid,
                name: name.to_string(),
                action: action.to_string(),
                avatar_path: avatar_path.map(|s| s.to_string()),
                sub_type: sub_type.to_string(),
            },
        );
    }

    rebuild_tray_menu(app_handle);
    reset_title_timer(app_handle.clone());
    info!("Tray notify: {} {} {}", name, action, rid);

    // ---- Sound playback ----
    let event_type = match (was_live, is_live) {
        (false, true) => "live",
        (true, false) => "offline",
        _ => return,
    };
    let streamer_name = name.to_string();
    let handle = app_handle.clone();

    if let Some(state) = handle.try_state::<Arc<AppState>>() {
        let data_dir = state.data_dir.clone();
        tauri::async_runtime::spawn(async move {
            let config = {
                if let Some(s) = handle.try_state::<Arc<AppState>>() {
                    let store = s.store.lock().unwrap();
                    store.get_all().config
                } else {
                    return;
                }
            };

            if config.sound_enabled {
                if let Err(e) = sound::play_random_for_streamer(
                    &streamer_name,
                    event_type,
                    config.sound_volume,
                    &data_dir,
                ) {
                    warn!("Sound playback failed: {}", e);
                }
            }
        });
    }
}

#[tauri::command]
async fn test_trigger_status(
    uid: u64,
    sub_type: String,
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
        cache.insert((sub_type.clone(), uid), new_status.clone()).unwrap_or(LiveStatus::Offline)
    };

    let name = {
        let store = state.store.lock().unwrap();
        let data = store.get_all();
        let sub = data.subscriptions.iter().find(|s| s.uid == uid && s.r#type == sub_type).ok_or("订阅不存在")?;
        sub.name.clone().unwrap_or_else(|| "未知".to_string())
    };

    let status_update = SubscriptionStatus {
        uid,
        sub_type: sub_type.clone(),
        name: name.clone(),
        status: new_status.clone(),
        title: if new_status == LiveStatus::Live {
            Some("【测试】模拟开播标题".to_string())
        } else {
            None
        },
        room_id: None,
        avatar_url: Some(state.avatar_full_path(&sub_type, uid)),
    };
    let _ = app_handle.emit("status-changed", &status_update);

    notify_status_change(
        &app_handle,
        uid,
        &sub_type,
        &name,
        &prev_status,
        &new_status,
        None,
        Some(&state.avatar_full_path(&sub_type, uid)),
    );

    Ok(())
}

#[tauri::command]
async fn set_autostart(
    enabled: bool,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // Update OS autostart
    if let Some(manager) = app_handle.try_state::<AutoLaunchManager>() {
        if enabled {
            manager.enable().map_err(|e| format!("Enable autostart: {}", e))?;
        } else {
            manager.disable().map_err(|e| format!("Disable autostart: {}", e))?;
        }
    }

    // Persist to store
    let mut store = state.store.lock().unwrap();
    store
        .set("config.autostart", enabled)
        .map_err(|e| format!("Store: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn set_show_window_on_startup(
    enabled: bool,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut store = state.store.lock().unwrap();
    store
        .set("config.show_window_on_startup", enabled)
        .map_err(|e| format!("Store: {}", e))?;
    Ok(())
}

// ---- App Entry Point ----

fn get_context<R: Runtime>() -> Context<R> {
    let mut context = tauri::generate_context!();
    if is_dev() {
        let config = context.config_mut();
        config.identifier += ".dev";
        if let Some(name) = &config.product_name {
            config.product_name = Some(name.to_owned() + ".dev");
        }
    }
    context
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let ctx = get_context();
    let identifier = ctx.config().identifier.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .app_name(&identifier)
                .build(),
        )
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_window(app);
        }))
        .setup(|app| {
            sound::init_audio();

            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));

            let state = Arc::new(AppState::new(data_dir));

            poller::start_poller(app.handle().clone(), state.clone());

            app.manage(state.clone());

            // Sync autostart state on launch
            {
                let store = state.store.lock().unwrap();
                let config = store.get_all().config.clone();
                if config.autostart {
                    if let Some(manager) = app.try_state::<AutoLaunchManager>() {
                        let _ = manager.enable();
                    }
                }
            }

            // Build tray
            let show_item = tauri::menu::MenuItemBuilder::with_id("show", "显示界面")
                .build(app)?;
            let quit_item = tauri::menu::MenuItemBuilder::with_id("quit", "退出")
                .build(app)?;
            let menu = tauri::menu::MenuBuilder::new(app)
                .item(&show_item)
                .item(&quit_item)
                .build()?;

            let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .expect("Failed to load tray icon");

            let app_handle = app.handle().clone();
            let _tray = tauri::tray::TrayIconBuilder::with_id("main")
                .icon(icon)
                .menu(&menu)
                .tooltip("推送姬")
                .on_menu_event(move |app_handle, event| {
                    let id = event.id().as_ref().to_string();
                    match id.as_str() {
                        "show" => show_window(app_handle),
                        "quit" => {
                            app_handle.exit(0);
                        }
                        "clear_all" => {
                            NOTIFS.lock().unwrap().clear();
                            rebuild_tray_menu(app_handle);
                        }
                        _ if id.starts_with("notif:") => {
                            let rid_str = id.strip_prefix("notif:").unwrap_or("0");
                            let rid: u64 = rid_str.parse().unwrap_or(0);
                            if rid != 0 {
                                let notifs = NOTIFS.lock().unwrap();
                                let url = if let Some(n) = notifs.iter().find(|n| n.room_id == rid) {
                                    match n.sub_type.as_str() {
                                        "huya" => api::huya::HuyaApi::room_url(rid),
                                        _ => api::bilibili::BilibiliApi::room_url(rid),
                                    }
                                } else {
                                    api::bilibili::BilibiliApi::room_url(rid)
                                };
                                drop(notifs);
                                let _ = app_handle.opener().open_url(&url, None::<&str>);
                            }
                            {
                                let mut notifs = NOTIFS.lock().unwrap();
                                notifs.retain(|n| n.room_id != rid);
                            }
                            rebuild_tray_menu(app_handle);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Create main window (visible based on config)
            let show_on_startup = {
                let store = state.store.lock().unwrap();
                store.get_all().config.show_window_on_startup
            };
            create_window(&app_handle, show_on_startup);

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
            update_badge_timeout,
            get_config,
            test_trigger_status,
            set_autostart,
            set_show_window_on_startup,
            download_streamer_sounds,
            get_sound_info,
            play_streamer_sound,
            set_sound_enabled,
            set_sound_volume,
            updater::restart_application,
        ])
        .run(ctx)
        .expect("error while running tauri application");
}
