use serde::Deserialize;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Deserialize, Debug)]
pub struct BilibiliResponse<T> {
    pub code: i64,
    #[allow(dead_code)]
    pub message: String,
    pub data: Option<T>,
}

// ---- Master/info endpoint ----

#[derive(Deserialize, Debug, Clone)]
pub struct MasterInfo {
    #[allow(dead_code)]
    pub uid: u64,
    pub uname: String,
    pub face: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct MasterInfoData {
    pub info: Option<MasterInfo>,
    pub room_id: Option<u64>,
}

/// Returns (user_name, avatar_url, room_id)
pub async fn get_master_info(uid: u64) -> Result<(String, Option<String>, u64), String> {
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
    let info = data.info.ok_or(format!("No user info found for UID {}", uid))?;
    let room_id = data.room_id.ok_or(format!(
        "UID {} has no live room (not a streamer?)",
        uid
    ))?;

    Ok((info.uname, info.face, room_id))
}

// ---- Room/get_info endpoint ----

#[derive(Deserialize, Debug, Clone)]
pub struct RoomInfo {
    #[allow(dead_code)]
    pub uid: u64,
    #[allow(dead_code)]
    pub room_id: u64,
    pub live_status: i64,
    pub title: String,
}

pub async fn get_room_info(room_id: u64) -> Result<RoomInfo, String> {
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

    body.data.ok_or("No room data in response".to_string())
}

// ---- Avatar download ----

pub async fn download_avatar(url: &str, uid: u64, data_dir: &std::path::Path) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Referer", "https://www.bilibili.com/")
        .send()
        .await
        .map_err(|e| format!("Avatar download failed: {}", e))?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Avatar read failed: {}", e))?;

    let avatars_dir = data_dir.join("avatars");
    std::fs::create_dir_all(&avatars_dir)
        .map_err(|e| format!("Failed to create avatars dir: {}", e))?;

    let file_path = avatars_dir.join(format!("{}.jpg", uid));
    std::fs::write(&file_path, &bytes)
        .map_err(|e| format!("Failed to save avatar: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}
