//! 下载队列：任务状态机 + 并发调度 + 进度事件 + 持久化（重启可续传）。
//!
//! 状态机: Queued → Downloading → Merging → Done / Failed / Canceled
//! 续传依赖 yt-dlp 的 `-c` 与 `.part` 文件；重启后未完成任务恢复为 Queued。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::oneshot;

use crate::engine::{ensure_engine, ffmpeg_path, new_command};
use crate::error::{classify_ytdlp_error, ApiError};
use crate::parser::validate_url;
use crate::settings::load_settings;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Queued,
    Downloading,
    Merging,
    Done,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    /// "mp4-1080" | "mp4-720" | "mp4-480" | "mp3" | "m4a"
    pub format: String,
    pub status: TaskStatus,
    pub percent: f64,
    /// 字节/秒
    pub speed: Option<f64>,
    /// 剩余秒数
    pub eta: Option<f64>,
    pub error: Option<String>,
    pub filepath: Option<String>,
}

#[derive(Default)]
pub struct DownloadState {
    tasks: Vec<Task>,
    kill_switches: HashMap<u64, oneshot::Sender<()>>,
    next_id: u64,
}

pub type SharedState = Arc<Mutex<DownloadState>>;

fn queue_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|d| d.join("queue.json"))
}

fn persist(app: &tauri::AppHandle, tasks: &[Task]) {
    if let Some(path) = queue_path(app) {
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        if let Ok(text) = serde_json::to_string_pretty(tasks) {
            let _ = std::fs::write(path, text);
        }
    }
}

/// 启动时加载持久化队列：进行中的任务恢复为排队（自动续传）。
pub fn load_persisted(app: &tauri::AppHandle) {
    let state = app.state::<SharedState>();
    let Some(path) = queue_path(app) else { return };
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let Ok(mut tasks) = serde_json::from_str::<Vec<Task>>(&text) else { return };
    for t in tasks.iter_mut() {
        if matches!(t.status, TaskStatus::Downloading | TaskStatus::Merging) {
            t.status = TaskStatus::Queued;
            t.speed = None;
            t.eta = None;
        }
    }
    let mut s = state.lock().unwrap();
    s.next_id = tasks.iter().map(|t| t.id).max().map_or(1, |m| m + 1);
    s.tasks = tasks;
}

fn emit_task(app: &tauri::AppHandle, task: &Task) {
    let _ = app.emit("task-updated", task.clone());
}

/// 格式串 → yt-dlp 参数（策略见 TubeGrab开发.md 第三节）。
/// "mp4-<高度>" 支持任意分辨率（如 mp4-2160）；"mp3" / "m4a" 为音频。
fn format_args(format: &str) -> Vec<String> {
    if format == "mp3" {
        return ["-f", "ba", "-x", "--audio-format", "mp3", "--audio-quality", "0"]
            .map(String::from)
            .to_vec();
    }
    if format == "m4a" {
        return ["-f", "ba[ext=m4a]/ba", "-x", "--audio-format", "m4a"]
            .map(String::from)
            .to_vec();
    }
    let height = format
        .strip_prefix("mp4-")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(1080);
    // 优先所选分辨率的 h264（兼容性最好）；该分辨率无 h264（如 4K 常只有 vp9）
    // 则接受同分辨率其他编码；再逐级回退到不超过所选分辨率的最优流
    vec![
        "-f".into(),
        format!(
            "bv*[height={height}][vcodec^=avc1]+ba[ext=m4a]/\
             bv*[height={height}]+ba[ext=m4a]/\
             bv*[height<={height}][vcodec^=avc1]+ba[ext=m4a]/\
             bv*[height<={height}]+ba[ext=m4a]/\
             b[height<={height}]"
        ),
        "--merge-output-format".into(),
        "mp4".into(),
    ]
}

/// 调度器：只要有空闲并发槽且有排队任务就启动下载。状态变化后都应调用。
pub fn pump(app: &tauri::AppHandle) {
    let state = app.state::<SharedState>().inner().clone();
    let concurrency = load_settings(app).map(|s| s.concurrency as usize).unwrap_or(2);

    loop {
        let started_id = {
            let mut s = state.lock().unwrap();
            let active = s
                .tasks
                .iter()
                .filter(|t| matches!(t.status, TaskStatus::Downloading | TaskStatus::Merging))
                .count();
            if active >= concurrency {
                return;
            }
            let Some(task) = s.tasks.iter_mut().find(|t| t.status == TaskStatus::Queued) else {
                return;
            };
            task.status = TaskStatus::Downloading;
            task.error = None;
            let id = task.id;
            let task_snapshot = task.clone();
            let (tx, rx) = oneshot::channel();
            s.kill_switches.insert(id, tx);
            drop(s);

            emit_task(app, &task_snapshot);
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                run_download(app2, id, rx).await;
            });
            id
        };
        let _ = started_id; // 继续循环，尝试填满剩余并发槽
    }
}

#[derive(serde::Deserialize)]
struct ProgressLine {
    status: Option<String>,
    downloaded_bytes: Option<f64>,
    total_bytes: Option<f64>,
    total_bytes_estimate: Option<f64>,
    speed: Option<f64>,
    eta: Option<f64>,
}

async fn run_download(app: tauri::AppHandle, id: u64, mut kill_rx: oneshot::Receiver<()>) {
    let state = app.state::<SharedState>().inner().clone();

    // 取任务快照与设置
    let (url, format) = {
        let s = state.lock().unwrap();
        let Some(t) = s.tasks.iter().find(|t| t.id == id) else { return };
        (t.url.clone(), t.format.clone())
    };
    let settings = match load_settings(&app) {
        Ok(s) => s,
        Err(e) => return fail_task(&app, id, e.to_string()),
    };

    let result: Result<Option<String>, ApiError> = async {
        let ytdlp = ensure_engine(&app)?;
        let ffmpeg = ffmpeg_path(&app)?;
        let out_dir = PathBuf::from(&settings.download_dir);
        std::fs::create_dir_all(&out_dir)
            .map_err(|e| ApiError::internal(format!("无法创建下载目录: {e}")))?;
        let outtmpl = out_dir.join(&settings.filename_template);

        let mut cmd = new_command(&ytdlp);
        cmd.args(format_args(&format))
            .args([
                "--no-playlist",
                "--no-warnings",
                "-c",
                "--newline",
                "--progress",
                "--progress-template",
                "%(progress)j",
                "--no-simulate",
                "--print",
                "after_move:FILEPATH::%(filepath)s",
                "--ffmpeg-location",
            ])
            .arg(&ffmpeg)
            .arg("-o")
            .arg(&outtmpl)
            .arg("--")
            .arg(&url)
            .env("PYTHONIOENCODING", "utf-8")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| ApiError::engine(format!("无法启动 yt-dlp: {e}")))?;

        // stderr 收集（用于失败时归类错误）
        let stderr_lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        if let Some(stderr) = child.stderr.take() {
            let sink = stderr_lines.clone();
            tauri::async_runtime::spawn(async move {
                let mut lines = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let mut v = sink.lock().unwrap();
                    v.push(line);
                    if v.len() > 40 {
                        v.remove(0);
                    }
                }
            });
        }

        let stdout = child.stdout.take().expect("stdout was piped");
        let mut lines = BufReader::new(stdout).lines();
        let mut filepath: Option<String> = None;
        let mut killed = false;
        let mut last_emit = Instant::now() - Duration::from_secs(1);

        loop {
            tokio::select! {
                line = lines.next_line() => {
                    match line {
                        Ok(Some(line)) => {
                            if let Some(rest) = line.strip_prefix("FILEPATH::") {
                                filepath = Some(rest.trim().to_string());
                                continue;
                            }
                            let Ok(p) = serde_json::from_str::<ProgressLine>(&line) else { continue };
                            let mut s = state.lock().unwrap();
                            let Some(t) = s.tasks.iter_mut().find(|t| t.id == id) else { break };
                            let status_str = p.status.as_deref().unwrap_or("");
                            match status_str {
                                "downloading" => {
                                    t.status = TaskStatus::Downloading;
                                    let total = p.total_bytes.or(p.total_bytes_estimate);
                                    if let (Some(db), Some(tb)) = (p.downloaded_bytes, total) {
                                        if tb > 0.0 {
                                            t.percent = (db / tb * 100.0).clamp(0.0, 100.0);
                                        }
                                    }
                                    t.speed = p.speed;
                                    t.eta = p.eta;
                                }
                                // 单条流下载完成；若还有后续流会回到 downloading，
                                // 否则保持"处理中"直到进程退出（合并/转码阶段）
                                "finished" => {
                                    t.status = TaskStatus::Merging;
                                    t.speed = None;
                                    t.eta = None;
                                }
                                _ => {}
                            }
                            let snapshot = t.clone();
                            drop(s);
                            if last_emit.elapsed() >= Duration::from_millis(200)
                                || snapshot.status != TaskStatus::Downloading
                            {
                                emit_task(&app, &snapshot);
                                last_emit = Instant::now();
                            }
                        }
                        _ => break,
                    }
                }
                _ = &mut kill_rx => {
                    let _ = child.start_kill();
                    killed = true;
                    break;
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| ApiError::internal(format!("等待子进程失败: {e}")))?;

        if killed {
            return Ok(None); // None 表示已取消
        }
        if !status.success() {
            let stderr = stderr_lines.lock().unwrap().join("\n");
            return Err(classify_ytdlp_error(&stderr));
        }
        Ok(Some(filepath.unwrap_or_default()))
    }
    .await;

    // 收尾：更新状态、清理 kill switch、持久化、推进队列
    {
        let mut s = state.lock().unwrap();
        s.kill_switches.remove(&id);
        if let Some(t) = s.tasks.iter_mut().find(|t| t.id == id) {
            match &result {
                Ok(Some(path)) => {
                    t.status = TaskStatus::Done;
                    t.percent = 100.0;
                    t.speed = None;
                    t.eta = None;
                    if !path.is_empty() {
                        t.filepath = Some(path.clone());
                    }
                }
                Ok(None) => {
                    t.status = TaskStatus::Canceled;
                    t.speed = None;
                    t.eta = None;
                }
                Err(e) => {
                    t.status = TaskStatus::Failed;
                    t.error = Some(e.message.clone());
                    t.speed = None;
                    t.eta = None;
                }
            }
            let snapshot = t.clone();
            let tasks = s.tasks.clone();
            drop(s);
            emit_task(&app, &snapshot);
            persist(&app, &tasks);
        }
    }
    pump(&app);
}

fn fail_task(app: &tauri::AppHandle, id: u64, message: String) {
    let state = app.state::<SharedState>().inner().clone();
    let mut s = state.lock().unwrap();
    s.kill_switches.remove(&id);
    if let Some(t) = s.tasks.iter_mut().find(|t| t.id == id) {
        t.status = TaskStatus::Failed;
        t.error = Some(message);
        let snapshot = t.clone();
        let tasks = s.tasks.clone();
        drop(s);
        emit_task(app, &snapshot);
        persist(app, &tasks);
    }
    pump(app);
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTaskInput {
    pub url: String,
    pub title: String,
    pub thumbnail: Option<String>,
    pub format: String,
}

#[tauri::command]
pub fn add_tasks(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    items: Vec<NewTaskInput>,
) -> Result<Vec<Task>, ApiError> {
    let mut created = Vec::new();
    {
        let mut s = state.lock().unwrap();
        for item in items {
            let url = validate_url(&item.url)?;
            let id = s.next_id;
            s.next_id += 1;
            let task = Task {
                id,
                url,
                title: item.title,
                thumbnail: item.thumbnail,
                format: item.format,
                status: TaskStatus::Queued,
                percent: 0.0,
                speed: None,
                eta: None,
                error: None,
                filepath: None,
            };
            s.tasks.push(task.clone());
            created.push(task);
        }
        persist(&app, &s.tasks);
    }
    for t in &created {
        emit_task(&app, t);
    }
    pump(&app);
    Ok(created)
}

#[tauri::command]
pub fn get_tasks(state: tauri::State<'_, SharedState>) -> Vec<Task> {
    state.lock().unwrap().tasks.clone()
}

#[tauri::command]
pub fn cancel_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    id: u64,
) -> Result<(), ApiError> {
    let mut s = state.lock().unwrap();
    if let Some(tx) = s.kill_switches.remove(&id) {
        let _ = tx.send(()); // 下载中：由 run_download 收尾并标记 Canceled
        return Ok(());
    }
    if let Some(t) = s.tasks.iter_mut().find(|t| t.id == id && t.status == TaskStatus::Queued) {
        t.status = TaskStatus::Canceled;
        let snapshot = t.clone();
        let tasks = s.tasks.clone();
        drop(s);
        emit_task(&app, &snapshot);
        persist(&app, &tasks);
    }
    Ok(())
}

#[tauri::command]
pub fn retry_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    id: u64,
) -> Result<(), ApiError> {
    {
        let mut s = state.lock().unwrap();
        let Some(t) = s.tasks.iter_mut().find(|t| t.id == id) else {
            return Err(ApiError::internal("任务不存在"));
        };
        if !matches!(t.status, TaskStatus::Failed | TaskStatus::Canceled) {
            return Err(ApiError::internal("仅失败或已取消的任务可重试"));
        }
        t.status = TaskStatus::Queued;
        t.error = None;
        t.percent = 0.0;
        let snapshot = t.clone();
        let tasks = s.tasks.clone();
        drop(s);
        emit_task(&app, &snapshot);
        persist(&app, &tasks);
    }
    pump(&app);
    Ok(())
}

#[tauri::command]
pub fn remove_task(
    app: tauri::AppHandle,
    state: tauri::State<'_, SharedState>,
    id: u64,
) -> Result<(), ApiError> {
    let mut s = state.lock().unwrap();
    if s.kill_switches.contains_key(&id) {
        return Err(ApiError::internal("任务进行中，请先取消"));
    }
    s.tasks.retain(|t| t.id != id);
    let tasks = s.tasks.clone();
    drop(s);
    persist(&app, &tasks);
    Ok(())
}

#[tauri::command]
pub fn open_in_folder(path: String) -> Result<(), ApiError> {
    tauri_plugin_opener::reveal_item_in_dir(&path)
        .map_err(|e| ApiError::internal(format!("无法打开文件夹: {e}")))
}
