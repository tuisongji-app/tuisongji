use async_trait::async_trait;
use scraper::{Html, Selector};

use super::PlatformApi;
use crate::state::LiveStatus;

pub struct HuyaApi;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// 从用户主页原始 HTML 中提取 roomId
fn extract_room_id(html: &str) -> Option<u64> {
    let marker = "\"roomId\":";
    let start = html.find(marker)? + marker.len();
    let slice = &html[start..];
    let end = slice.find(',')?;
    slice[..end].trim().parse().ok()
}

/// 从房间页面提取真实用户 UID（.host-pic href 中的 /video/u/12345）
fn extract_uid_from_host_pic(html: &str) -> Option<u64> {
    let marker = "www.huya.com/video/u/";
    let start = html.find(marker)? + marker.len();
    let slice = &html[start..];
    let end = slice.find(['"', '\'', '/', ' '])?;
    slice[..end].parse().ok()
}

#[async_trait]
impl PlatformApi for HuyaApi {
    /// 从用户主页获取主播信息
    /// https://www.huya.com/video/u/{uid}
    async fn get_master_info(platform_id: u64) -> Result<(u64, String, Option<String>, u64), String> {
        let url = format!("https://www.huya.com/video/u/{}", platform_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("HTTP 请求失败: {}", e))?;

        let html = resp
            .text()
            .await
            .map_err(|e| format!("读取页面失败: {}", e))?;

        let document = Html::parse_document(&html);

        // 提取昵称：.detail-nick span
        let nick = Selector::parse(".detail-nick span")
            .ok()
            .and_then(|sel| document.select(&sel).next())
            .map(|el| el.text().collect::<String>().trim().to_string())
            .unwrap_or_else(|| "未知".to_string());

        // 提取头像：.content-avatar img
        let avatar = Selector::parse(".content-avatar img")
            .ok()
            .and_then(|sel| document.select(&sel).next())
            .and_then(|el| el.value().attr("src"))
            .map(|s| s.to_string());

        // 提取 roomId：正则匹配原始 HTML 中的 "roomId":数字
        let room_id = extract_room_id(&html)
            .ok_or(format!("该用户 (UID {}) 没有直播间", platform_id))?;

        if room_id == 0 {
            return Err(format!("该用户 (UID {}) 没有直播间", platform_id));
        }

        Ok((platform_id, nick, avatar, room_id))
    }

    /// 通过房间号获取主播信息（直接从房间页面提取）
    async fn get_master_info_by_room(room_id: u64) -> Result<(u64, String, Option<String>, u64), String> {
        let url = format!("https://www.huya.com/{}", room_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("HTTP 请求失败: {}", e))?;

        let html = resp
            .text()
            .await
            .map_err(|e| format!("读取页面失败: {}", e))?;

        let document = Html::parse_document(&html);

        // 提取昵称：.host-name 的 title 属性
        let nick = Selector::parse(".host-name")
            .ok()
            .and_then(|sel| document.select(&sel).next())
            .and_then(|el| el.value().attr("title"))
            .unwrap_or("未知")
            .to_string();

        // 提取头像：.host-pic img 的 src 属性
        let avatar = Selector::parse(".host-pic img")
            .ok()
            .and_then(|sel| document.select(&sel).next())
            .and_then(|el| el.value().attr("src"))
            .map(|s| {
                if s.starts_with("//") {
                    format!("https:{}", s)
                } else {
                    s.to_string()
                }
            });

        // 提取真实用户 UID（从 .host-pic href）
        let resolved_uid = extract_uid_from_host_pic(&html).unwrap_or(room_id);

        if nick == "未知" && avatar.is_none() {
            return Err(format!("房间 {} 不存在或无法访问", room_id));
        }

        Ok((resolved_uid, nick, avatar, room_id))
    }

    async fn get_room_info(room_id: u64) -> Result<(i64, String), String> {
        let url = format!("https://www.huya.com/{}", room_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("User-Agent", USER_AGENT)
            .send()
            .await
            .map_err(|e| format!("HTTP 请求失败: {}", e))?;

        let html = resp
            .text()
            .await
            .map_err(|e| format!("读取页面失败: {}", e))?;

        // 检测直播状态：gameLiveInfo 中的 isOn 字段
        // 直播中: "isOn":true, state:"ON", totalCount > 0
        // 离线:   "isOn":false, state:"OFF", totalCount = 0
        let live_status: i64 = if html.contains("\"isOn\":true") { 1 } else { 0 };

        let document = Html::parse_document(&html);

        // 提取房间标题：.host-title 的 title 属性
        let title = Selector::parse(".host-title")
            .ok()
            .and_then(|sel| document.select(&sel).next())
            .and_then(|el| el.value().attr("title"))
            .unwrap_or("")
            .to_string();

        Ok((live_status, title))
    }

    fn map_live_status(code: i64) -> LiveStatus {
        match code {
            1 => LiveStatus::Live,
            _ => LiveStatus::Offline,
        }
    }

    fn room_url(room_id: u64) -> String {
        format!("https://www.huya.com/{}", room_id)
    }

    fn platform_name() -> &'static str {
        "huya"
    }
}
