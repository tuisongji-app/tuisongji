pub mod bilibili;
pub mod douyu;
pub mod huya;

use async_trait::async_trait;
use std::path::Path;

use crate::state::LiveStatus;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[async_trait]
pub trait PlatformApi {
    /// 解析主播信息：platform_id → (resolved_uid, 昵称, 头像URL, room_id)
    async fn get_master_info(platform_id: u64) -> Result<(u64, String, Option<String>, u64), String>;

    /// 通过房间号解析主播信息：room_id → (resolved_uid, 昵称, 头像URL, room_id)
    async fn get_master_info_by_room(room_id: u64) -> Result<(u64, String, Option<String>, u64), String>;

    /// 获取直播间状态：room_id → (live_status, 标题)
    async fn get_room_info(room_id: u64) -> Result<(i64, String), String>;

    /// 将平台特定的状态码映射为通用 LiveStatus
    fn map_live_status(code: i64) -> LiveStatus;

    /// 直播间网页 URL（托盘通知点击时打开）
    fn room_url(room_id: u64) -> String;

    /// 平台名称标识（对应 Subscription.r#type）
    #[allow(dead_code)]
    fn platform_name() -> &'static str;
}

/// 下载头像到 {data_dir}/avatars/{sub_type}/{uid}.jpg
pub async fn download_avatar(
    url: &str,
    uid: u64,
    sub_type: &str,
    data_dir: &Path,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Avatar download failed: {}", e))?;

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Avatar read failed: {}", e))?;

    let avatars_dir = data_dir.join("avatars").join(sub_type);
    std::fs::create_dir_all(&avatars_dir)
        .map_err(|e| format!("Failed to create avatars dir: {}", e))?;

    let file_path = avatars_dir.join(format!("{}.jpg", uid));
    std::fs::write(&file_path, &bytes)
        .map_err(|e| format!("Failed to save avatar: {}", e))?;

    Ok(file_path.to_string_lossy().to_string())
}
