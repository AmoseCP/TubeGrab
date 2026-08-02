//! 应用自身的更新：启动时查询 GitHub Releases 最新版本，
//! Windows 安装版支持应用内下载并启动安装程序；便携版与 macOS 引导到下载页。

use serde::Serialize;
use tauri::Emitter;

use crate::error::ApiError;

const RELEASES_API: &str = "https://api.github.com/repos/AmoseCP/TubeGrab/releases/latest";
const RELEASES_PAGE: &str = "https://github.com/AmoseCP/TubeGrab/releases/latest";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppUpdateInfo {
    pub available: bool,
    pub current: String,
    pub latest: String,
    pub notes: String,
    pub page_url: String,
    /// 能否应用内自动安装（Windows 安装版且找到 setup.exe 资产）
    pub can_auto_install: bool,
    pub asset_url: Option<String>,
}

/// "v0.2.0" -> [0, 2, 0]；非数字后缀（如 -beta）截断忽略。
fn parse_ver(v: &str) -> Vec<u64> {
    v.trim_start_matches(['v', 'V'])
        .split(['.', '-'])
        .map_while(|p| p.parse::<u64>().ok())
        .collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let l = parse_ver(latest);
    !l.is_empty() && l > parse_ver(current)
}

/// 安装版的安装目录中有 NSIS 生成的 uninstall.exe，便携版没有。
fn is_installed_windows() -> bool {
    if !cfg!(windows) {
        return false;
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("uninstall.exe").exists()))
        .unwrap_or(false)
}

fn http_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .user_agent("TubeGrab")
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| ApiError::internal(format!("初始化 HTTP 客户端失败: {e}")))
}

async fn fetch_latest_release() -> Result<serde_json::Value, ApiError> {
    let resp = http_client()?
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(20))
        .send()
        .await
        .map_err(|e| ApiError::network(format!("检查更新失败: {e}")))?;
    if !resp.status().is_success() {
        return Err(ApiError::network(format!(
            "检查更新失败: GitHub 返回 {}",
            resp.status()
        )));
    }
    resp.json()
        .await
        .map_err(|e| ApiError::network(format!("解析更新信息失败: {e}")))
}

#[tauri::command]
pub async fn check_app_update(app: tauri::AppHandle) -> Result<AppUpdateInfo, ApiError> {
    let current = app.package_info().version.to_string();
    let release = fetch_latest_release().await?;
    let latest = release["tag_name"].as_str().unwrap_or("").to_string();
    let notes = release["body"].as_str().unwrap_or("").to_string();
    let page_url = release["html_url"]
        .as_str()
        .unwrap_or(RELEASES_PAGE)
        .to_string();
    let asset_url = release["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| {
            a["name"]
                .as_str()
                .is_some_and(|n| n.contains("x64") && n.ends_with("-setup.exe"))
        })
        .and_then(|a| a["browser_download_url"].as_str())
        .map(String::from);
    Ok(AppUpdateInfo {
        available: is_newer(&latest, &current),
        current,
        latest,
        notes,
        page_url,
        can_auto_install: is_installed_windows() && asset_url.is_some(),
        asset_url,
    })
}

/// 下载安装程序到临时目录（进度经 app-update-progress 事件推送），
/// 然后启动安装程序并退出本应用（避免文件占用）。
#[tauri::command]
pub async fn install_app_update(app: tauri::AppHandle, url: String) -> Result<(), ApiError> {
    use futures_util::StreamExt;
    use tokio::io::AsyncWriteExt;

    let resp = http_client()?
        .get(&url)
        .send()
        .await
        .map_err(|e| ApiError::network(format!("下载更新失败: {e}")))?;
    if !resp.status().is_success() {
        return Err(ApiError::network(format!(
            "下载更新失败: 服务器返回 {}",
            resp.status()
        )));
    }
    let total = resp.content_length().unwrap_or(0);
    let path = std::env::temp_dir().join("TubeGrab-update-setup.exe");
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|e| ApiError::internal(format!("创建临时文件失败: {e}")))?;

    let mut stream = resp.bytes_stream();
    let mut downloaded: u64 = 0;
    let mut last_pct: u64 = u64::MAX;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| ApiError::network(format!("下载中断: {e}")))?;
        file.write_all(&chunk)
            .await
            .map_err(|e| ApiError::internal(format!("写入临时文件失败: {e}")))?;
        downloaded += chunk.len() as u64;
        let pct = if total > 0 { downloaded * 100 / total } else { 0 };
        if pct != last_pct {
            last_pct = pct;
            let _ = app.emit(
                "app-update-progress",
                serde_json::json!({ "downloaded": downloaded, "total": total }),
            );
        }
    }
    file.flush()
        .await
        .map_err(|e| ApiError::internal(format!("写入临时文件失败: {e}")))?;
    drop(file);

    std::process::Command::new(&path)
        .spawn()
        .map_err(|e| ApiError::internal(format!("启动安装程序失败: {e}")))?;
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare() {
        assert!(is_newer("v0.3.0", "0.2.0"));
        assert!(is_newer("v0.10.0", "0.9.1"));
        assert!(is_newer("1.0.0", "0.99.99"));
        assert!(is_newer("v0.2.1", "0.2.0"));
        assert!(!is_newer("v0.2.0", "0.2.0"));
        assert!(!is_newer("v0.2.0", "0.3.0"));
        assert!(!is_newer("", "0.1.0"));
    }

    /// 联网冒烟测试：确认 GitHub API 返回结构与解析逻辑匹配。
    #[tokio::test]
    async fn fetch_latest_release_shape() {
        let release = fetch_latest_release().await.expect("API 请求失败");
        let tag = release["tag_name"].as_str().expect("缺少 tag_name");
        assert!(!parse_ver(tag).is_empty(), "tag 无法解析: {tag}");
        let setup = release["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|a| {
                a["name"]
                    .as_str()
                    .is_some_and(|n| n.contains("x64") && n.ends_with("-setup.exe"))
            });
        assert!(setup, "release 中找不到 x64 setup.exe 资产");
    }
}
