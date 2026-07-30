//! 设置的读写（appdata/settings.json）。

use std::path::PathBuf;
use tauri::Manager;

use crate::error::ApiError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub download_dir: String,
    pub concurrency: u32,
    pub filename_template: String,
    /// "mp4-1080" | "mp4-720" | "mp4-480" | "mp3" | "m4a"
    pub default_format: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: String::new(), // 空表示未初始化，加载时填充系统下载目录
            concurrency: 2,
            filename_template: "%(title)s.%(ext)s".to_string(),
            default_format: "mp4-1080".to_string(),
        }
    }
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, ApiError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ApiError::internal(format!("无法定位应用数据目录: {e}")))?;
    Ok(dir.join("settings.json"))
}

pub fn load_settings(app: &tauri::AppHandle) -> Result<Settings, ApiError> {
    let path = settings_path(app)?;
    let mut settings: Settings = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => Settings::default(),
    };
    if settings.download_dir.is_empty() {
        let downloads = app
            .path()
            .download_dir()
            .map(|d| d.join("TubeGrab"))
            .unwrap_or_else(|_| PathBuf::from("TubeGrab"));
        settings.download_dir = downloads.to_string_lossy().to_string();
    }
    if settings.concurrency == 0 {
        settings.concurrency = 1;
    }
    Ok(settings)
}

#[tauri::command]
pub fn get_settings(app: tauri::AppHandle) -> Result<Settings, ApiError> {
    load_settings(&app)
}

#[tauri::command]
pub fn save_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), ApiError> {
    let path = settings_path(&app)?;
    std::fs::create_dir_all(path.parent().unwrap())
        .map_err(|e| ApiError::internal(format!("创建设置目录失败: {e}")))?;
    let text = serde_json::to_string_pretty(&settings)
        .map_err(|e| ApiError::internal(format!("序列化设置失败: {e}")))?;
    std::fs::write(&path, text).map_err(|e| ApiError::internal(format!("写入设置失败: {e}")))?;
    // 并发数可能改变，唤醒调度器
    crate::download::pump(&app);
    Ok(())
}
