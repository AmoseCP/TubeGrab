//! yt-dlp / ffmpeg 的寻址、首次启动复制与自更新。
//!
//! yt-dlp 运行副本放在应用数据目录（安装目录通常不可写，无法自更新），
//! 首次启动时从捆绑资源复制过去；ffmpeg 无需更新，直接从资源目录运行。

use std::path::PathBuf;
use tauri::path::BaseDirectory;
use tauri::Manager;
use tokio::process::Command;

use crate::error::ApiError;

#[cfg(windows)]
pub const YTDLP_NAME: &str = "yt-dlp.exe";
#[cfg(not(windows))]
pub const YTDLP_NAME: &str = "yt-dlp";

#[cfg(windows)]
pub const FFMPEG_NAME: &str = "ffmpeg.exe";
#[cfg(not(windows))]
pub const FFMPEG_NAME: &str = "ffmpeg";

/// 子进程统一创建入口：Windows 下隐藏控制台窗口。
pub fn new_command(program: &PathBuf) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    cmd
}

/// 应用数据目录中的 yt-dlp 运行副本路径。
pub fn ytdlp_path(app: &tauri::AppHandle) -> Result<PathBuf, ApiError> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| ApiError::internal(format!("无法定位应用数据目录: {e}")))?
        .join("engine");
    Ok(dir.join(YTDLP_NAME))
}

/// 捆绑资源中的 ffmpeg 路径（yt-dlp 通过 --ffmpeg-location 使用）。
pub fn ffmpeg_path(app: &tauri::AppHandle) -> Result<PathBuf, ApiError> {
    app.path()
        .resolve(format!("binaries/{FFMPEG_NAME}"), BaseDirectory::Resource)
        .map_err(|e| ApiError::internal(format!("无法定位捆绑的 ffmpeg: {e}")))
}

/// 确保应用数据目录中存在 yt-dlp 副本（首次启动时从资源复制）。
pub fn ensure_engine(app: &tauri::AppHandle) -> Result<PathBuf, ApiError> {
    let target = ytdlp_path(app)?;
    if !target.exists() {
        let bundled = app
            .path()
            .resolve(format!("binaries/{YTDLP_NAME}"), BaseDirectory::Resource)
            .map_err(|e| ApiError::internal(format!("无法定位捆绑的 yt-dlp: {e}")))?;
        std::fs::create_dir_all(target.parent().unwrap())
            .map_err(|e| ApiError::internal(format!("创建引擎目录失败: {e}")))?;
        std::fs::copy(&bundled, &target)
            .map_err(|e| ApiError::internal(format!("复制 yt-dlp 失败: {e}")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755));
        }
    }
    Ok(target)
}

async fn run_version(program: &PathBuf, args: &[&str]) -> Result<String, ApiError> {
    let output = new_command(program)
        .args(args)
        .output()
        .await
        .map_err(|e| ApiError::engine(format!("无法启动 {}: {e}", program.display())))?;
    if !output.status.success() {
        return Err(ApiError::engine(format!(
            "{} 退出异常: {}",
            program.display(),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.lines().next().unwrap_or("").trim().to_string())
}

#[derive(serde::Serialize)]
pub struct EngineInfo {
    pub ytdlp_version: String,
    pub ffmpeg_version: String,
}

#[tauri::command]
pub async fn get_engine_version(app: tauri::AppHandle) -> Result<EngineInfo, ApiError> {
    let ytdlp = ensure_engine(&app)?;
    let ffmpeg = ffmpeg_path(&app)?;
    let ytdlp_version = run_version(&ytdlp, &["--version"]).await?;
    // ffmpeg -version 首行形如 "ffmpeg version N-xxxx-... Copyright ..."，取版本字段
    let ffmpeg_line = run_version(&ffmpeg, &["-version"]).await?;
    let ffmpeg_version = ffmpeg_line
        .split_whitespace()
        .nth(2)
        .unwrap_or(&ffmpeg_line)
        .to_string();
    Ok(EngineInfo {
        ytdlp_version,
        ffmpeg_version,
    })
}

/// 引擎一键更新：官方独立二进制自带 `-U` 自更新（带签名校验），
/// 更新的是应用数据目录中的运行副本。
#[tauri::command]
pub async fn update_engine(app: tauri::AppHandle) -> Result<String, ApiError> {
    let ytdlp = ensure_engine(&app)?;
    let output = new_command(&ytdlp)
        .arg("-U")
        .output()
        .await
        .map_err(|e| ApiError::engine(format!("无法启动 yt-dlp: {e}")))?;
    if !output.status.success() {
        return Err(ApiError::engine(format!(
            "引擎更新失败: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    // 更新完成后返回新版本号
    run_version(&ytdlp, &["--version"]).await
}
