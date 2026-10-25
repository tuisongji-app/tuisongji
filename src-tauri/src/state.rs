use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Mutex;
use store::figment::value::Value;
use store::Store;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub r#type: String,
    pub uid: u64,
    pub name: Option<String>,
    pub room_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveStatus {
    Offline,
    Live,
    Replay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub poll_interval_mins: u64,
    pub badge_timeout_mins: u64,
    pub autostart: bool,
    pub show_window_on_startup: bool,
    pub sound_enabled: bool,
    pub sound_volume: f32,
    pub auto_check_update: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            poll_interval_mins: 30,
            badge_timeout_mins: 30,
            autostart: false,
            show_window_on_startup: true,
            sound_enabled: true,
            sound_volume: 0.8,
            auto_check_update: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionStatus {
    pub uid: u64,
    pub sub_type: String,
    pub name: String,
    pub status: LiveStatus,
    pub title: Option<String>,
    pub room_id: Option<u64>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SoundInfo {
    pub name: String,
    pub available_live: u32,
    pub available_offline: u32,
    pub downloaded_live: u32,
    pub downloaded_offline: u32,
    pub live_files: Vec<String>,
    pub offline_files: Vec<String>,
    pub downloaded_live_files: Vec<String>,
    pub downloaded_offline_files: Vec<String>,
}

/// Persisted state — managed by Store
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PersistData {
    pub subscriptions: Vec<Subscription>,
    pub config: AppConfig,
}

pub struct AppState {
    pub store: Mutex<Store<PersistData>>,
    pub status_cache: Mutex<HashMap<(String, u64), LiveStatus>>,
    pub data_dir: PathBuf,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let path = data_dir.join("state.json");
        let path_str = path.to_string_lossy().to_string();
        let store = Store::<PersistData>::builder()
            .set_save_path(&path_str, store::FileFormat::Json)
            .set_default_source(PersistData::default())
            .expect("Failed to set default source")
            .build()
            .expect("Failed to initialize store");

        Self {
            store: Mutex::new(store),
            status_cache: Mutex::new(HashMap::new()),
            data_dir,
        }
    }

    pub fn avatar_full_path(&self, sub_type: &str, uid: u64) -> String {
        self.data_dir
            .join("avatars")
            .join(sub_type)
            .join(format!("{}.jpg", uid))
            .to_string_lossy()
            .to_string()
    }

    pub fn remove_subscription(&self, uid: u64, sub_type: &str) -> Result<(), String> {
        // Keep the store locked through cleanup so another subscription cannot
        // start sharing this sound directory while it is being removed.
        let mut store = self.store.lock().unwrap();
        let mut data = store.get_all();
        let Some(removed) = data
            .subscriptions
            .iter()
            .find(|sub| sub.uid == uid && sub.r#type == sub_type)
            .cloned()
        else {
            return Ok(());
        };

        data.subscriptions
            .retain(|sub| !(sub.uid == uid && sub.r#type == sub_type));
        let value =
            Value::serialize(&data.subscriptions).map_err(|e| format!("Serialize: {}", e))?;

        // Sounds are stored by display name and may be shared across platforms.
        let sounds_dir = match removed.name.as_deref() {
            Some(name)
                if !data
                    .subscriptions
                    .iter()
                    .any(|sub| sub.name.as_deref() == Some(name)) =>
            {
                Some(crate::sound::streamer_sounds_dir(name, &self.data_dir)?)
            }
            _ => None,
        };
        if !matches!(sub_type, "bilibili" | "huya" | "douyu") {
            return Err("无效的订阅平台".to_string());
        }

        if let Some(dir) = sounds_dir {
            match fs::remove_dir_all(&dir) {
                Ok(()) => {}
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => return Err(format!("删除音效文件夹失败：{}", e)),
            }
        }
        match fs::remove_file(self.avatar_full_path(sub_type, uid)) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(format!("删除头像失败：{}", e)),
        }

        store
            .set("subscriptions", value)
            .map_err(|e| format!("Store: {}", e))?;
        self.status_cache
            .lock()
            .unwrap()
            .remove(&(sub_type.to_string(), uid));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    struct Fixture {
        state: AppState,
    }

    impl Fixture {
        fn new(subscriptions: Vec<Subscription>) -> Self {
            static NEXT_ID: AtomicU64 = AtomicU64::new(0);
            let dir = std::env::temp_dir().join(format!(
                "tuisongji-subscriptions-{}-{}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).unwrap();
            let state = AppState::new(dir);
            state
                .store
                .lock()
                .unwrap()
                .set("subscriptions", Value::serialize(&subscriptions).unwrap())
                .unwrap();
            for sub in subscriptions {
                let avatar = PathBuf::from(state.avatar_full_path(&sub.r#type, sub.uid));
                fs::create_dir_all(avatar.parent().unwrap()).unwrap();
                fs::write(avatar, b"avatar").unwrap();
                if let Some(name) = sub.name {
                    for event in ["live", "offline"] {
                        let dir =
                            crate::sound::event_sounds_dir(&name, event, &state.data_dir).unwrap();
                        fs::create_dir_all(&dir).unwrap();
                        fs::write(dir.join("audio.wav"), b"audio").unwrap();
                    }
                }
                state
                    .status_cache
                    .lock()
                    .unwrap()
                    .insert((sub.r#type, sub.uid), LiveStatus::Live);
            }
            Self { state }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.state.data_dir);
        }
    }

    fn subscription(uid: u64, sub_type: &str, name: &str) -> Subscription {
        Subscription {
            uid,
            r#type: sub_type.to_string(),
            name: Some(name.to_string()),
            room_id: Some(uid),
        }
    }

    #[test]
    fn removal_deletes_owned_files_and_persists_remaining_subscriptions() {
        let fixture = Fixture::new(vec![
            subscription(1, "bilibili", "主播"),
            subscription(2, "huya", "其他主播"),
        ]);
        let state = &fixture.state;
        let source = state.data_dir.join("original.wav");
        fs::write(&source, b"original audio").unwrap();

        state.remove_subscription(1, "bilibili").unwrap();

        assert!(!state.data_dir.join("sounds/主播").exists());
        assert!(!PathBuf::from(state.avatar_full_path("bilibili", 1)).exists());
        assert!(!state
            .status_cache
            .lock()
            .unwrap()
            .contains_key(&("bilibili".into(), 1)));
        assert!(state
            .data_dir
            .join("sounds/其他主播/live/audio.wav")
            .exists());
        assert!(PathBuf::from(state.avatar_full_path("huya", 2)).exists());
        assert!(source.exists());
        let reloaded = AppState::new(state.data_dir.clone());
        let subs = reloaded.store.lock().unwrap().get_all().subscriptions;
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].uid, 2);
    }

    #[test]
    fn removal_keeps_shared_audio_until_the_last_subscription_is_removed() {
        let fixture = Fixture::new(vec![
            subscription(1, "bilibili", "同名主播"),
            subscription(1, "huya", "同名主播"),
        ]);
        let state = &fixture.state;

        state.remove_subscription(1, "bilibili").unwrap();
        assert!(state
            .data_dir
            .join("sounds/同名主播/live/audio.wav")
            .exists());
        assert!(state
            .data_dir
            .join("sounds/同名主播/offline/audio.wav")
            .exists());
        assert!(PathBuf::from(state.avatar_full_path("huya", 1)).exists());
        assert!(state
            .status_cache
            .lock()
            .unwrap()
            .contains_key(&("huya".into(), 1)));

        state.remove_subscription(1, "huya").unwrap();
        assert!(!state.data_dir.join("sounds/同名主播").exists());
    }

    #[test]
    fn removal_succeeds_when_files_are_missing_and_can_be_repeated() {
        let fixture = Fixture::new(vec![subscription(1, "bilibili", "主播")]);
        let state = &fixture.state;
        fs::remove_dir_all(state.data_dir.join("sounds")).unwrap();
        fs::remove_dir_all(state.data_dir.join("avatars")).unwrap();

        state.remove_subscription(1, "bilibili").unwrap();
        state.remove_subscription(1, "bilibili").unwrap();
        assert!(state
            .store
            .lock()
            .unwrap()
            .get_all()
            .subscriptions
            .is_empty());
    }

    #[test]
    fn cleanup_errors_keep_the_subscription_available_for_retry() {
        let fixture = Fixture::new(vec![subscription(1, "bilibili", "主播")]);
        let state = &fixture.state;
        let dir = state.data_dir.join("sounds/主播");
        fs::remove_dir_all(&dir).unwrap();
        // A file in place of the directory causes a deterministic cleanup error.
        fs::write(&dir, b"unexpected file").unwrap();

        let error = state.remove_subscription(1, "bilibili").unwrap_err();
        assert!(error.contains("删除音效文件夹失败"));
        assert_eq!(state.store.lock().unwrap().get_all().subscriptions.len(), 1);
        assert!(PathBuf::from(state.avatar_full_path("bilibili", 1)).exists());

        fs::remove_file(dir).unwrap();
        state.remove_subscription(1, "bilibili").unwrap();
        assert!(state
            .store
            .lock()
            .unwrap()
            .get_all()
            .subscriptions
            .is_empty());
    }
}
