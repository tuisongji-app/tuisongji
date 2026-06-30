use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub uid: u64,
    pub name: Option<String>,
    pub room_id: Option<u64>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LiveStatus {
    Offline,
    Live,
    Replay,
}

impl LiveStatus {
    pub fn from_i64(v: i64) -> Self {
        match v {
            1 => LiveStatus::Live,
            2 => LiveStatus::Replay,
            _ => LiveStatus::Offline,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub poll_interval_secs: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            poll_interval_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionStatus {
    pub uid: u64,
    pub name: String,
    pub status: LiveStatus,
    pub title: Option<String>,
    pub room_id: Option<u64>,
    pub avatar_url: Option<String>,
}

pub struct AppState {
    pub subscriptions: Mutex<Vec<Subscription>>,
    pub status_cache: Mutex<HashMap<u64, LiveStatus>>,
    pub config: Mutex<AppConfig>,
    pub data_dir: PathBuf,
}

#[derive(Serialize, Deserialize)]
struct PersistData {
    subscriptions: Vec<Subscription>,
    config: AppConfig,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let state = Self {
            subscriptions: Mutex::new(Vec::new()),
            status_cache: Mutex::new(HashMap::new()),
            config: Mutex::new(AppConfig::default()),
            data_dir,
        };
        state.load();
        state
    }

    fn persist_path(&self) -> PathBuf {
        self.data_dir.join("subscriptions.json")
    }

    fn load(&self) {
        let path = self.persist_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(data) = serde_json::from_str::<PersistData>(&content) {
                    if let Ok(mut subs) = self.subscriptions.lock() {
                        *subs = data.subscriptions;
                    }
                    if let Ok(mut cfg) = self.config.lock() {
                        *cfg = data.config;
                    }
                }
            }
        }
    }

    pub fn save(&self) {
        let path = self.persist_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let data = PersistData {
            subscriptions: self
                .subscriptions
                .lock()
                .map(|s| s.clone())
                .unwrap_or_default(),
            config: self.config.lock().map(|c| c.clone()).unwrap_or_default(),
        };

        if let Ok(json) = serde_json::to_string_pretty(&data) {
            let _ = fs::write(&path, json);
        }
    }
}
