# TubeGrab 详细开发计划（执行版）

> 依据 `TubeGrab开发.md` 的完整需求制定。遵循 CLAUDE.md 准则：每一步有明确的验证标准，只实现需求中列出的功能，不做投机性设计。

## 0. 关键技术决策（含与原文档的差异说明）

| 决策 | 内容 | 理由 |
|------|------|------|
| 包管理器 | npm（本机未装 pnpm） | 与 pnpm 无功能差异 |
| 开发平台 | Windows 优先开发验证；macOS 通过 GitHub Actions 构建配置支持 | 本机为 Windows，无 Mac 真机 |
| 二进制捆绑方式 | Tauri `bundle.resources`（`src-tauri/binaries/`），**不用** sidecar/externalBin | 引擎需要"一键更新"覆盖自身，安装目录（Program Files）不可写。方案：首次启动把 yt-dlp 复制到应用数据目录，之后一律从数据目录运行；更新时直接覆盖数据目录副本。ffmpeg 不需更新，从资源目录只读运行，用 `--ffmpeg-location` 传给 yt-dlp |
| 子进程 | `tokio::process::Command` + Windows `CREATE_NO_WINDOW`，逐行读 stdout | 异步不阻塞；避免闪黑窗 |
| 格式选择 | 固定预设（1080p/720p/480p MP4、MP3、M4A），解析结果只用于展示信息和禁用不可用档位 | 文档第三节的格式策略即预设,避免用户面对上百个原始 format |
| 队列 | Rust 端 `Mutex<状态>` + 并发上限调度，任务列表持久化到 `queue.json` | 满足"退出重开可续传" |
| 进度协议 | `--newline --progress-template "%(progress)j"`，解析 JSON 行，Tauri event 推送 | 文档指定 |

## 1. 数据流与模块

```
React UI (App.tsx + 组件)
  │ invoke: get_engine_version / parse_url / parse_playlist /
  │         add_tasks / retry_task / cancel_task / remove_task /
  │         get_tasks / get_settings / save_settings / update_engine /
  │         open_in_folder
  │ listen: task-updated (任务状态/进度), engine-update-progress
  ▼
Rust (src-tauri/src/)
  ├─ engine.rs    寻址/首启复制/版本查询/GitHub 更新
  ├─ parser.rs    yt-dlp -J 解析（单视频 & --flat-playlist）
  ├─ download.rs  任务状态机 + 队列调度 + 进度事件 + 持久化
  ├─ settings.rs  设置读写（appdata/settings.json）
  └─ lib.rs       组装 commands / state
```

任务状态机：`Queued → Downloading → Merging → Done / Failed / Canceled`（失败可 Retry 回 Queued；续传靠 yt-dlp `-c` 与 `.part` 文件）。

## 2. 阶段与验收标准

### Phase 0：骨架（验收：App 启动显示 yt-dlp 与 ffmpeg 版本号）
1. create-tauri-app 脚手架（React+TS+Vite），合入现有目录
2. 下载 yt-dlp.exe 与 ffmpeg（BtbN win64 静态构建）→ `src-tauri/binaries/`（加入 .gitignore，另写下载脚本 `scripts/fetch-binaries.ps1` 供重建）
3. `bundle.resources` 配置 + engine.rs（首启复制到 appdata）+ `get_engine_version`
4. 前端启动时显示版本
5. 验证：`npm run tauri dev` 启动、显示真实版本号；`cargo check` 无警告级错误

### Phase 1：解析与单任务下载（验收：完整走通 1080p MP4 与 MP3 下载）
1. `parse_url`：`yt-dlp -J --no-playlist <url>`，返回 {title, thumbnail, duration, 可用高度列表}；URL 白名单校验（http/https）
2. 格式预设 → -f 参数映射（文档第三节原文）
3. `start/add task` + 进度事件（percent/speed/eta/status）
4. UI：输入框 → 解析卡片（封面/标题/时长/格式下拉）→ 下载 → 进度条 → 完成后"打开所在文件夹"
5. 错误处理：无效链接 / 网络失败 / 引擎报错 → 结构化错误 + 友好提示（引擎报错时提示可能需要更新引擎）
6. 验证：真实链接下载 1080p MP4 和 MP3 各一个，文件可打开

### Phase 2：队列与播放列表（验收：批量下载稳定完成；重开 App 可续传）
1. 队列调度：并发数取自设置（默认 2），先进先出
2. `parse_playlist`：`--flat-playlist -J`，列出条目 + 勾选批量加入
3. 失败重试按钮；`-c --no-part-overwrite` 续传；`queue.json` 持久化未完成任务，启动时恢复为 Queued
4. 验证：播放列表批量下载；下载中退出重开，任务恢复并续传

### Phase 3：设置与引擎更新（验收：更新后版本号变化且下载正常）
1. 设置页：下载目录（默认系统 Downloads/TubeGrab）、并发数、命名模板（默认 `%(title)s.%(ext)s`）、默认格式
2. `update_engine`：GitHub API 取 latest release → 下载对应平台资产 → 覆盖 appdata 副本 → 返回新版本号
3. 解析失败的错误提示中带"更新引擎"入口
4. 验证：更新引擎 → 版本变化 → 再跑一次下载

### Phase 4：打包与 CI（验收：本机 NSIS 安装包安装后可用；CI 配置齐全）
1. Windows：`npm run tauri build` 产出 NSIS 安装包，本机验证
2. macOS 打包配置（Intel + Apple Silicon 二进制命名、chmod/quarantine 说明写入 README）
3. GitHub Actions：win + mac 双平台构建 workflow（含按平台拉取二进制）
4. README：运行/测试/打包/引擎更新说明 + 免责声明

## 3. 明确不做（v1）
登录内容、直播、内置播放器、代理设置、macOS 签名公证（无账号，README 说明右键打开绕过 Gatekeeper）。
