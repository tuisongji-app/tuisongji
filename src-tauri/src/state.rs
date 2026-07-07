use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
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

    #[allow(dead_code)]
    pub fn streamer_sounds_dir(&self, name: &str) -> PathBuf {
        self.data_dir.join("sounds").join(name)
    }
}
