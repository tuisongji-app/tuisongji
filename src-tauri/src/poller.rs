use crate::bilibili_api;
use crate::state::{AppState, LiveStatus};
use log::{error, warn};
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;

pub fn start_poller(app_handle: AppHandle, state: Arc<AppState>) {
    tauri::async_runtime::spawn(async move {
        // Initial delay to let app settle
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        loop {
            let interval_secs = {
                let config = state.config.lock().unwrap();
                config.poll_interval_secs
            };

            let subscriptions = {
                let subs = state.subscriptions.lock().unwrap();
                subs.clone()
            };

            for (i, sub) in subscriptions.iter().enumerate() {
                if i > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }

                // Use cached room_id if available, otherwise resolve from master info
                let room_id = match sub.room_id {
                    Some(rid) => rid,
                    None => {
                        // First-time resolution: get room_id from master info
                        match bilibili_api::get_master_info(sub.uid).await {
                            Ok((name, avatar_url, rid)) => {
                                let mut subs = state.subscriptions.lock().unwrap();
                                if let Some(s) = subs.iter_mut().find(|s| s.uid == sub.uid) {
                                    s.name = Some(name);
                                    s.room_id = Some(rid);
                                    s.avatar_url = avatar_url;
                                }
                                drop(subs);
                                state.save();
                                rid
                            }
                            Err(e) => {
                                error!("Poll resolve error for uid {}: {}", sub.uid, e);
                                continue;
                            }
                        }
                    }
                };

                // Query room info for live status
                match bilibili_api::get_room_info(room_id).await {
                    Ok(room_info) => {
                        let new_status = LiveStatus::from_i64(room_info.live_status);

                        // Refresh user name if still unknown
                        let needs_name_refresh = {
                            let subs = state.subscriptions.lock().unwrap();
                            subs.iter()
                                .find(|s| s.uid == sub.uid)
                                .map(|s| s.name.is_none() || s.name.as_deref() == Some("未知"))
                                .unwrap_or(false)
                        };
                        if needs_name_refresh {
                            let data_dir = state.data_dir.clone();
                            if let Ok((name, remote_avatar, _)) =
                                bilibili_api::get_master_info(sub.uid).await
                            {
                                let local_avatar = if let Some(ref url) = remote_avatar {
                                    bilibili_api::download_avatar(url, sub.uid, &data_dir)
                                        .await
                                        .ok()
                                } else {
                                    None
                                };
                                let mut subs = state.subscriptions.lock().unwrap();
                                if let Some(s) = subs.iter_mut().find(|s| s.uid == sub.uid) {
                                    s.name = Some(name);
                                    s.avatar_url = local_avatar;
                                }
                                drop(subs);
                                state.save();
                            }
                        }

                        // Check for status change
                        let prev_status = {
                            let mut cache = state.status_cache.lock().unwrap();
                            cache
                                .insert(sub.uid, new_status.clone())
                                .unwrap_or(LiveStatus::Offline)
                        };

                        if prev_status != new_status {
                            let display_name = {
                                let subs = state.subscriptions.lock().unwrap();
                                subs.iter()
                                    .find(|s| s.uid == sub.uid)
                                    .and_then(|s| s.name.clone())
                                    .unwrap_or_else(|| "未知".to_string())
                            };

                            let avatar_url = {
                                let subs = state.subscriptions.lock().unwrap();
                                subs.iter()
                                    .find(|s| s.uid == sub.uid)
                                    .and_then(|s| s.avatar_url.clone())
                            };

                            // Emit event to frontend
                            let status_update = crate::state::SubscriptionStatus {
                                uid: sub.uid,
                                name: display_name.clone(),
                                status: new_status.clone(),
                                title: Some(room_info.title.clone()),
                                room_id: Some(room_id),
                                avatar_url,
                            };
                            let _ = app_handle.emit("status-changed", &status_update);

                            // Notify via shared function
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

            tokio::time::sleep(std::time::Duration::from_secs(interval_secs)).await;
        }
    });
}
