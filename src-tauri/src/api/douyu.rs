use async_trait::async_trait;
use serde::Deserialize;

use super::PlatformApi;
use crate::state::LiveStatus;

pub struct DouyuApi;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Debug, Deserialize)]
struct DouyuBetardResponse {
    room: Option<DouyuRoom>,
    owner_avatar: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DouyuRoom {
    #[allow(dead_code)]
    room_id: u64,
    room_name: String,
    nickname: String,
    owner_uid: u64,
    show_status: i64,
    avatar: Option<DouyuAvatar>,
}

#[derive(Debug, Deserialize)]
struct DouyuAvatar {
    big: Option<String>,
    middle: Option<String>,
    #[allow(dead_code)]
    small: Option<String>,
}

/// 从 betard 响应中提取头像 URL，优先使用顶层 owner_avatar，其次 room.avatar
fn pick_avatar(resp: &DouyuBetardResponse) -> Option<String> {
    if let Some(ref owner_avatar) = resp.owner_avatar {
        if !owner_avatar.is_empty() {
            return Some(owner_avatar.clone());
        }
    }
    if let Some(ref room) = resp.room {
        if let Some(ref avatar) = room.avatar {
            if let Some(ref big) = avatar.big {
                if !big.is_empty() {
                    return Some(big.clone());
                }
            }
            if let Some(ref middle) = avatar.middle {
                if !middle.is_empty() {
                    return Some(middle.clone());
                }
            }
        }
    }
    None
}

/// 调用 betard API 获取房间数据
async fn fetch_betard(room_id: u64) -> Result<DouyuBetardResponse, String> {
    let url = format!("https://www.douyu.com/betard/{}", room_id);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .header("Referer", "https://www.douyu.com/")
        .send()
        .await
        .map_err(|e| format!("HTTP 请求失败: {}", e))?;

    let text = resp
        .text()
        .await
        .map_err(|e| format!("读取响应失败: {}", e))?;

    serde_json::from_str::<DouyuBetardResponse>(&text)
        .map_err(|e| format!("解析 JSON 失败: {}", e))
}

#[async_trait]
impl PlatformApi for DouyuApi {
    /// 斗鱼不支持通过 UID 查询主播信息
    async fn get_master_info(_platform_id: u64) -> Result<(u64, String, Option<String>, u64), String> {
        Err("斗鱼仅支持房间号查询，请使用房间号添加".to_string())
    }

    /// 通过房间号获取主播信息
    async fn get_master_info_by_room(room_id: u64) -> Result<(u64, String, Option<String>, u64), String> {
        let data = fetch_betard(room_id).await?;

        let room = data
            .room
            .as_ref()
            .ok_or(format!("房间 {} 不存在或无法访问", room_id))?;

        let nickname = room.nickname.clone();
        let owner_uid = room.owner_uid;
        let avatar = pick_avatar(&data);

        Ok((owner_uid, nickname, avatar, room_id))
    }

    /// 获取直播间状态
    async fn get_room_info(room_id: u64) -> Result<(i64, String), String> {
        let data = fetch_betard(room_id).await?;

        let room = data
            .room
            .as_ref()
            .ok_or(format!("房间 {} 不存在或无法访问", room_id))?;

        Ok((room.show_status, room.room_name.clone()))
    }

    fn map_live_status(code: i64) -> LiveStatus {
        match code {
            1 => LiveStatus::Live,
            _ => LiveStatus::Offline,
        }
    }

    fn room_url(room_id: u64) -> String {
        format!("https://www.douyu.com/{}", room_id)
    }

    fn platform_name() -> &'static str {
        "douyu"
    }
}
