use crate::bilibili_api;
use crate::state::{AppState, LiveStatus};
use log::{error, warn};
use std::sync::Arc;
use store::figment::value::Value;
use tauri::AppHandle;
use tauri::Emitter;

pub fn start_poller(app_handle: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

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
                        match bilibili_api::get_master_info(sub.uid).await {
                            Ok((name, remote_avatar, rid)) => {
                                // Persist resolved info
                                {
                                    let mut store = state.store.lock().unwrap();
                                    let mut data = store.get_all();
                                    if let Some(s) =
                                        data.subscriptions.iter_mut().find(|s| s.uid == sub.uid)
                                    {
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
                                    let _ =
                                        bilibili_api::download_avatar(url, sub.uid, &data_dir).await;
                                }
                                rid
                            }
                            Err(e) => {
                                error!("Poll resolve error for uid {}: {}", sub.uid, e);
                                continue;
                            }
                        }
                    }
                };

                match bilibili_api::get_room_info(room_id).await {
                    Ok(room_info) => {
                        let new_status = LiveStatus::from_i64(room_info.live_status);

                        // Refresh user name if still unknown
                        let needs_name_refresh = {
                            let store = state.store.lock().unwrap();
                            let data = store.get_all();
                            data.subscriptions
                                .iter()
                                .find(|s| s.uid == sub.uid)
                                .map(|s| s.name.is_none() || s.name.as_deref() == Some("未知"))
                                .unwrap_or(false)
                        };
                        if needs_name_refresh {
                            if let Ok((name, remote_avatar, _)) =
                                bilibili_api::get_master_info(sub.uid).await
                            {
                                let data_dir = state.data_dir.clone();
                                if let Some(ref url) = remote_avatar {
                                    let _ = bilibili_api::download_avatar(url, sub.uid, &data_dir)
                                        .await;
                                }
                                let mut store = state.store.lock().unwrap();
                                let mut data = store.get_all();
                                if let Some(s) =
                                    data.subscriptions.iter_mut().find(|s| s.uid == sub.uid)
                                {
                                    s.name = Some(name);
                                }
                                if let Ok(v) = Value::serialize(&data.subscriptions) {
                                    let _ = store.set("subscriptions", v);
                                }
                            }
                        }

                        let prev_status = {
                            let mut cache = state.status_cache.lock().unwrap();
                            cache
                                .insert(sub.uid, new_status.clone())
                                .unwrap_or(LiveStatus::Offline)
                        };

                        if prev_status != new_status {
                            let display_name = {
                                let store = state.store.lock().unwrap();
                                let data = store.get_all();
                                data.subscriptions
                                    .iter()
                                    .find(|s| s.uid == sub.uid)
                                    .and_then(|s| s.name.clone())
                                    .unwrap_or_else(|| "未知".to_string())
                            };

                            let status_update = crate::state::SubscriptionStatus {
                                uid: sub.uid,
                                name: display_name.clone(),
                                status: new_status.clone(),
                                title: Some(room_info.title.clone()),
                                room_id: Some(room_id),
                                avatar_url: Some(state.avatar_full_path(sub.uid)),
                            };
                            let _ = app_handle.emit("status-changed", &status_update);

                            crate::notify_status_change(
                                &app_handle,
                                &display_name,
                                &prev_status,
                                &new_status,
                                Some(&room_info.title),
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Poll failed for uid {}: {}", sub.uid, e);
                    }
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(interval_secs * 60)).await;
        }
    });
}
