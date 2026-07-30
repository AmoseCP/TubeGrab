//! 链接解析：一切通过 yt-dlp -J，绝不自己实现解析逻辑。

use crate::engine::{ensure_engine, new_command};
use crate::error::{classify_ytdlp_error, ApiError};

/// URL 校验：仅接受 http/https 且不含空白/控制字符。
/// （参数以数组传递本身即安全，此校验为额外防线。）
pub fn validate_url(url: &str) -> Result<String, ApiError> {
    let url = url.trim();
    if url.is_empty() {
        return Err(ApiError::invalid_url("请输入链接"));
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(ApiError::invalid_url("链接必须以 http:// 或 https:// 开头"));
    }
    if url.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(ApiError::invalid_url("链接包含非法字符"));
    }
    Ok(url.to_string())
}

async fn run_json(app: &tauri::AppHandle, args: &[&str]) -> Result<serde_json::Value, ApiError> {
    let ytdlp = ensure_engine(app)?;
    let output = new_command(&ytdlp)
        .args(args)
        .output()
        .await
        .map_err(|e| ApiError::engine(format!("无法启动 yt-dlp: {e}")))?;
    if !output.status.success() {
        return Err(classify_ytdlp_error(&String::from_utf8_lossy(&output.stderr)));
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| ApiError::engine(format!("解析引擎输出失败: {e}")))
}

/// 某一档分辨率的信息（码率 kbps、预估大小 bytes，均可能未知）
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoOption {
    pub height: u32,
    pub tbr: Option<f64>,
    pub filesize: Option<f64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioInfo {
    /// 最佳音频流码率 kbps
    pub abr: Option<f64>,
    pub filesize: Option<f64>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    pub uploader: Option<String>,
    /// 源里实际可用的分辨率档位（降序）
    pub video_options: Vec<VideoOption>,
    pub audio: AudioInfo,
}

#[tauri::command]
pub async fn parse_url(app: tauri::AppHandle, url: String) -> Result<VideoInfo, ApiError> {
    let url = validate_url(&url)?;
    let json = run_json(&app, &["-J", "--no-playlist", "--no-warnings", &url]).await?;

    let empty = Vec::new();
    let formats = json["formats"].as_array().unwrap_or(&empty);

    let size_of = |f: &serde_json::Value| f["filesize"].as_f64().or(f["filesize_approx"].as_f64());

    // 最佳音频流（vcodec=none 的纯音频，取码率最高者）
    let mut audio = AudioInfo { abr: None, filesize: None };
    for f in formats {
        let is_audio = f["vcodec"].as_str() == Some("none")
            && f["acodec"].as_str().map_or(false, |a| a != "none");
        if !is_audio {
            continue;
        }
        let abr = f["abr"].as_f64();
        if abr.unwrap_or(0.0) >= audio.abr.unwrap_or(0.0) {
            audio.abr = abr.or(audio.abr);
            audio.filesize = size_of(f).or(audio.filesize);
        }
    }

    // 每档分辨率取最优视频流：优先 h264(avc1)（下载策略同款），同编码取码率高者
    use std::collections::BTreeMap;
    let mut by_height: BTreeMap<u32, (bool, f64, Option<f64>)> = BTreeMap::new();
    for f in formats {
        let has_video = f["vcodec"].as_str().map_or(false, |v| v != "none");
        let Some(height) = f["height"].as_u64().map(|h| h as u32) else { continue };
        if !has_video || height == 0 || f["ext"].as_str() == Some("mhtml") {
            continue;
        }
        let is_avc = f["vcodec"].as_str().map_or(false, |v| v.starts_with("avc1"));
        let tbr = f["tbr"].as_f64().unwrap_or(0.0);
        let entry = by_height.entry(height).or_insert((is_avc, tbr, size_of(f)));
        if (is_avc, tbr) > (entry.0, entry.1) {
            *entry = (is_avc, tbr, size_of(f));
        }
    }
    // 预估大小 = 视频流 + 音频流（合并后近似值）
    let video_options: Vec<VideoOption> = by_height
        .iter()
        .rev()
        .map(|(&height, &(_, tbr, size))| VideoOption {
            height,
            tbr: (tbr > 0.0).then_some(tbr),
            filesize: match (size, audio.filesize) {
                (Some(v), Some(a)) => Some(v + a),
                (v, _) => v,
            },
        })
        .collect();

    Ok(VideoInfo {
        url,
        title: json["title"].as_str().unwrap_or("未知标题").to_string(),
        thumbnail: json["thumbnail"].as_str().map(String::from),
        duration: json["duration"].as_f64(),
        uploader: json["uploader"].as_str().map(String::from),
        video_options,
        audio,
    })
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistEntry {
    pub url: String,
    pub title: String,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaylistInfo {
    pub title: String,
    pub entries: Vec<PlaylistEntry>,
}

#[tauri::command]
pub async fn parse_playlist(app: tauri::AppHandle, url: String) -> Result<PlaylistInfo, ApiError> {
    let url = validate_url(&url)?;
    let json = run_json(&app, &["-J", "--flat-playlist", "--no-warnings", &url]).await?;

    let entries = json["entries"]
        .as_array()
        .ok_or_else(|| ApiError::invalid_url("该链接不是播放列表"))?
        .iter()
        .filter_map(|e| {
            let entry_url = e["url"]
                .as_str()
                .map(String::from)
                .or_else(|| e["id"].as_str().map(|id| format!("https://www.youtube.com/watch?v={id}")))?;
            Some(PlaylistEntry {
                url: entry_url,
                title: e["title"].as_str().unwrap_or("未知标题").to_string(),
                duration: e["duration"].as_f64(),
                thumbnail: e["thumbnails"][0]["url"].as_str().map(String::from),
            })
        })
        .collect();

    Ok(PlaylistInfo {
        title: json["title"].as_str().unwrap_or("播放列表").to_string(),
        entries,
    })
}
