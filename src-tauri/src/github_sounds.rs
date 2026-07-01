use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

const REPO_OWNER: &str = "tuisongji-app";
const REPO_NAME: &str = "sound";
const RAW_BASE: &str = "https://raw.githubusercontent.com";

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

#[derive(Deserialize, Debug, Clone)]
pub struct Sounds {
    pub live: Vec<String>,
    pub offline: Vec<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Manifest {
    pub channels: HashMap<String, Sounds>,
}

fn raw_url(path: &str) -> String {
    format!("{}/{}/{}/main/{}", RAW_BASE, REPO_OWNER, REPO_NAME, path)
}

/// Fetch the manifest from GitHub.
pub async fn fetch_manifest() -> Result<Manifest, String> {
    let client = reqwest::Client::new();
    let url = raw_url("manifest.json");
    let resp = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("Manifest fetch HTTP {}", resp.status()));
    }

    resp.json::<Manifest>()
        .await
        .map_err(|e| format!("Failed to parse manifest: {}", e))
}

/// Download all missing sound files for a given streamer name.
/// Returns `(downloaded_live_count, downloaded_offline_count)`.
pub async fn download_sounds_for_name(
    name: &str,
    data_dir: &Path,
) -> Result<(u32, u32), String> {
    let manifest = fetch_manifest().await?;

    let sounds = manifest
        .channels
        .get(name)
        .ok_or_else(|| format!("该主播（{}）暂无可用音效", name))?;

    let base_dir = data_dir.join("sounds").join(name);
    let client = reqwest::Client::new();

    let mut dl_live = 0u32;
    let mut dl_offline = 0u32;

    // Download live sounds
    let live_dir = base_dir.join("live");
    for file in &sounds.live {
        if try_download(&client, name, "live", file, &live_dir).await? {
            dl_live += 1;
        }
    }

    // Download offline sounds
    let offline_dir = base_dir.join("offline");
    for file in &sounds.offline {
        if try_download(&client, name, "offline", file, &offline_dir).await? {
            dl_offline += 1;
        }
    }

    Ok((dl_live, dl_offline))
}

/// Returns the available sound counts for a streamer from the manifest.
/// Returns `(live_count, offline_count)`.
pub async fn available_sounds_for_name(name: &str) -> Result<(u32, u32), String> {
    let manifest = fetch_manifest().await?;
    match manifest.channels.get(name) {
        Some(s) => Ok((s.live.len() as u32, s.offline.len() as u32)),
        None => Ok((0, 0)),
    }
}

/// Try to download a single sound file. Returns `Ok(true)` if downloaded,
/// `Ok(false)` if skipped (already exists). Returns `Err` on failure.
async fn try_download(
    client: &reqwest::Client,
    name: &str,
    event_type: &str,
    filename: &str,
    dir: &Path,
) -> Result<bool, String> {
    let file_path = dir.join(filename);

    // Duplicate detection: skip if already exists
    if file_path.exists() {
        return Ok(false);
    }

    // Ensure directory exists
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("Failed to create dir {}: {}", dir.display(), e))?;

    let url = raw_url(&format!("sounds/{}/{}/{}", name, event_type, filename));
    let resp = client
        .get(&url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| format!("Download failed for {}: {}", filename, e))?;

    if !resp.status().is_success() {
        return Err(format!("Download {} HTTP {}", filename, resp.status()));
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| format!("Read failed for {}: {}", filename, e))?;

    std::fs::write(&file_path, &bytes)
        .map_err(|e| format!("Save failed for {}: {}", filename, e))?;

    Ok(true)
}
