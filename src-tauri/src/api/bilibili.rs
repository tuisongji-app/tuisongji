use async_trait::async_trait;
use serde::Deserialize;

use super::{PlatformApi, USER_AGENT};
use crate::state::LiveStatus;

pub struct BilibiliApi;

// ---- Response wrappers ----

#[derive(Deserialize, Debug)]
struct BilibiliResponse<T> {
    code: i64,
    #[allow(dead_code)]
    message: String,
    data: Option<T>,
}

// ---- Master/info endpoint ----

#[derive(Deserialize, Debug, Clone)]
struct MasterInfo {
    #[allow(dead_code)]
    uid: u64,
    uname: String,
    face: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
struct MasterInfoData {
    info: Option<MasterInfo>,
    room_id: Option<u64>,
}

// ---- Room/get_info endpoint ----

#[derive(Deserialize, Debug, Clone)]
struct RoomInfo {
    #[allow(dead_code)]
    uid: u64,
    #[allow(dead_code)]
    room_id: u64,
    live_status: i64,
    title: String,
}

#[async_trait]
impl PlatformApi for BilibiliApi {
    async fn get_master_info(platform_id: u64) -> Result<(u64, String, Option<String>, u64), String> {
        let uid = platform_id;
        let url = format!(
            "https://api.live.bilibili.com/live_user/v1/Master/info?uid={}",
            uid
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", "https://live.bilibili.com/")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let body: BilibiliResponse<MasterInfoData> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if body.code != 0 {
            return Err(format!(
                "Bilibili API error: code={}, {}",
                body.code, body.message
            ));
        }

        let data = body.data.ok_or("No data in response")?;
        let info = data
            .info
            .ok_or(format!("No user info found for UID {}", uid))?;
        let room_id = data.room_id.ok_or(format!(
            "UID {} has no live room (not a streamer?)",
            uid
        ))?;

        Ok((uid, info.uname, info.face, room_id))
    }

    async fn get_room_info(room_id: u64) -> Result<(i64, String), String> {
        let url = format!(
            "https://api.live.bilibili.com/room/v1/Room/get_info?room_id={}",
            room_id
        );

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", "https://live.bilibili.com/")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let body: BilibiliResponse<RoomInfo> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if body.code != 0 {
            return Err(format!(
                "Bilibili API error: code={}, {}",
                body.code, body.message
            ));
        }

        let data = body.data.ok_or("No room data in response".to_string())?;
        Ok((data.live_status, data.title))
    }

    async fn get_master_info_by_room(room_id: u64) -> Result<(u64, String, Option<String>, u64), String> {
        // 先通过房间号获取 uid
        let url = format!(
            "https://api.live.bilibili.com/room/v1/Room/get_info?room_id={}",
            room_id
        );
        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .header("Referer", "https://live.bilibili.com/")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {}", e))?;

        let body: BilibiliResponse<RoomInfo> = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if body.code != 0 {
            return Err(format!(
                "Bilibili API error: code={}, {}",
                body.code, body.message
            ));
        }

        let room_data = body.data.ok_or("No room data in response".to_string())?;
        let uid = room_data.uid;

        // 再用 uid 获取主播信息
        Self::get_master_info(uid).await
    }

    fn map_live_status(code: i64) -> LiveStatus {
        match code {
            1 => LiveStatus::Live,
            2 => LiveStatus::Replay,
            _ => LiveStatus::Offline,
        }
    }

    fn room_url(room_id: u64) -> String {
        format!("https://live.bilibili.com/{}", room_id)
    }

    fn platform_name() -> &'static str {
        "bilibili"
    }
}
