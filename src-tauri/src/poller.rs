use crate::api;
use crate::api::PlatformApi;
use crate::state::{AppState, LiveStatus, Subscription};
use log::{error, warn};
use std::sync::Arc;
use store::figment::value::Value;
use tauri::AppHandle;
use tauri::Emitter;

pub fn start_poller(app_handle: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {

        loop {
            let (interval_secs, subscriptions) = {
                let store = state.store.lock().unwrap();
                let data = store.get_all();
                (data.config.poll_interval_mins, data.subscriptions)
            };

            for (i, sub) in subscriptions.iter().enumerate() {
                if i > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }

                let room_id = match sub.room_id {
                    Some(rid) => rid,
                    None => {
                        let resolve_result = match sub.r#type.as_str() {
                            "bilibili" => api::bilibili::BilibiliApi::get_master_info(sub.uid).await,
                            "huya" => api::huya::HuyaApi::get_master_info(sub.uid).await,
                            "douyu" => api::douyu::DouyuApi::get_master_info(sub.uid).await,
                            _ => {
                                error!("未知平台类型: {}", sub.r#type);
                                continue;
                            }
                        };

                        match resolve_result {
                            Ok((resolved_uid, name, remote_avatar, rid)) => {
                                // Persist resolved info
                                let resolved_id = resolved_uid;
                                {
                                    let mut store = state.store.lock().unwrap();
                                    let mut data = store.get_all();
                                    if let Some(s) = data.subscriptions.iter_mut().find(|s| {
                                        s.uid == sub.uid && s.r#type == sub.r#type
                                    }) {
                                        s.uid = resolved_id;
                                        s.name = Some(name);
                                        s.room_id = Some(rid);
                                    }
                                    if let Ok(v) = Value::serialize(&data.subscriptions) {
                                        let _ = store.set("subscriptions", v);
                                    }
                                }
                                // Download avatar (lock dropped, safe to await)
                                if let Some(ref url) = remote_avatar {
                                    let data_dir = state.data_dir.clone();
                                    let sub_type = sub.r#type.clone();
                                    let _ = api::download_avatar(url, resolved_id, &sub_type, &data_dir).await;
                                }
                                rid
                            }
                            Err(e) => {
                                error!("Poll resolve error for uid {} ({}): {}", sub.uid, sub.r#type, e);
                                continue;
                            }
                        }
                    }
                };

                let room_result = match sub.r#type.as_str() {
                    "bilibili" => api::bilibili::BilibiliApi::get_room_info(room_id).await,
                    "huya" => api::huya::HuyaApi::get_room_info(room_id).await,
                    "douyu" => api::douyu::DouyuApi::get_room_info(room_id).await,
                    _ => {
                        warn!("未知平台类型: {}", sub.r#type);
                        continue;
                    }
                };

                match room_result {
                    Ok((live_status_code, title)) => {
                        let new_status = match sub.r#type.as_str() {
                            "bilibili" => api::bilibili::BilibiliApi::map_live_status(live_status_code),
                            "huya" => api::huya::HuyaApi::map_live_status(live_status_code),
                            "douyu" => api::douyu::DouyuApi::map_live_status(live_status_code),
                            _ => LiveStatus::Offline,
                        };

                        // Refresh user name if still unknown
                        let needs_name_refresh = {
                            let store = state.store.lock().unwrap();
                            let data = store.get_all();
                            data.subscriptions
                                .iter()
                                .find(|s| s.uid == sub.uid && s.r#type == sub.r#type)
                                .map(|s| s.name.is_none() || s.name.as_deref() == Some("未知"))
                                .unwrap_or(false)
                        };
                        if needs_name_refresh {
                            let refresh_result = match sub.r#type.as_str() {
                                "bilibili" => api::bilibili::BilibiliApi::get_master_info(sub.uid).await,
                                "huya" => api::huya::HuyaApi::get_master_info(sub.uid).await,
                                "douyu" => api::douyu::DouyuApi::get_master_info(sub.uid).await,
                                _ => Err("未知平台".to_string()),
                            };
                            if let Ok((_resolved_uid, name, remote_avatar, _)) = refresh_result {
                                let data_dir = state.data_dir.clone();
                                if let Some(ref url) = remote_avatar {
                                    let sub_type = sub.r#type.clone();
                                    let _ = api::download_avatar(url, sub.uid, &sub_type, &data_dir)
                                        .await;
                                }
                                let mut store = state.store.lock().unwrap();
                                let mut data = store.get_all();
                                if let Some(s) = data.subscriptions.iter_mut().find(|s| {
                                    s.uid == sub.uid && s.r#type == sub.r#type
                                }) {
                                    s.name = Some(name);
                                }
                                if let Ok(v) = Value::serialize(&data.subscriptions) {
                                    let _ = store.set("subscriptions", v);
                                }
                            }
                        }

                        let cache_key = (sub.r#type.clone(), sub.uid);
                        let prev = {
                            let mut cache = state.status_cache.lock().unwrap();
                            cache.insert(cache_key, new_status.clone())
                        };

                        // Collect display name (needed for both first-poll emit
                        // and change-notification emit).
                        let display_name = {
                            let store = state.store.lock().unwrap();
                            let data = store.get_all();
                            let s = data
                                .subscriptions
                                .iter()
                                .find(|s| s.uid == sub.uid && s.r#type == sub.r#type)
                                .cloned()
                                .unwrap_or(Subscription {
                                    r#type: sub.r#type.clone(),
                                    uid: sub.uid,
                                    name: Some("未知".into()),
                                    room_id: None,
                                });
                            s.name.unwrap_or_else(|| "未知".to_string())
                        };

                        match prev {
                            Some(old) if old != new_status => {
                                // Real status change — notify.
                                let status_update = crate::state::SubscriptionStatus {
                                    uid: sub.uid,
                                    sub_type: sub.r#type.clone(),
                                    name: display_name.clone(),
                                    status: new_status.clone(),
                                    title: Some(title.clone()),
                                    room_id: Some(room_id),
                                    avatar_url: Some(state.avatar_full_path(&sub.r#type, sub.uid)),
                                };
                                let _ = app_handle.emit("status-changed", &status_update);

                                crate::notify_status_change(
                                    &app_handle,
                                    sub.uid,
                                    &sub.r#type,
                                    &display_name,
                                    &old,
                                    &new_status,
                                    Some(room_id),
                                    Some(&state.avatar_full_path(&sub.r#type, sub.uid)),
                                );
                            }
                            Some(_) => {
                                // Same status — nothing to do.
                            }
                            None => {
                                // First poll — treat as transition from Offline.
                                let old = LiveStatus::Offline;
                                if old != new_status {
                                    let status_update = crate::state::SubscriptionStatus {
                                        uid: sub.uid,
                                        sub_type: sub.r#type.clone(),
                                        name: display_name.clone(),
                                        status: new_status.clone(),
                                        title: Some(title.clone()),
                                        room_id: Some(room_id),
                                        avatar_url: Some(state.avatar_full_path(&sub.r#type, sub.uid)),
                                    };
                                    let _ = app_handle.emit("status-changed", &status_update);

                                    crate::notify_status_change(
                                        &app_handle,
                                        sub.uid,
                                        &sub.r#type,
                                        &display_name,
                                        &old,
                                        &new_status,
                                        Some(room_id),
                                        Some(&state.avatar_full_path(&sub.r#type, sub.uid)),
                                    );
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Poll failed for uid {} ({}): {}", sub.uid, sub.r#type, e);
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval_secs * 60)).await;
        }
    });
}
