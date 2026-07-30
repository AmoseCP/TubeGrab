# TubeGrab

跨平台（Windows / macOS）YouTube 视频/音频下载工具。粘贴链接即可下载视频（MP4 1080p/720p/480p）或音频（MP3 / M4A），支持播放列表批量下载、下载队列、断点续传与引擎一键更新。

- 架构：Tauri 2（Rust 后端） + React + TypeScript + Tailwind（前端）
- 下载引擎：捆绑 [yt-dlp](https://github.com/yt-dlp/yt-dlp) 独立二进制（不自己实现任何解析逻辑）
- 后处理：捆绑 [ffmpeg](https://ffmpeg.org/) 静态构建（DASH 音视频流合并、MP3 转码）

## 开发环境要求

- Node.js ≥ 20（npm）
- Rust ≥ 1.88（stable，`rustup update stable`）
- Windows：MSVC 构建工具 + WebView2（Win10/11 一般自带）
- macOS：Xcode Command Line Tools

## 首次准备：拉取引擎二进制

二进制文件（约 160MB）不入库，构建前先下载到 `src-tauri/binaries/`：

```powershell
# Windows
powershell -ExecutionPolicy Bypass -File scripts/fetch-binaries.ps1
```

```bash
# macOS（可选参数 arm64 / x64，默认取本机架构）
bash scripts/fetch-binaries.sh
```

## 运行（开发模式）

```bash
npm install
npm run tauri dev
```

> 注：Vite 开发端口用的是 **15420**（Windows 上默认的 1420 常落入 Hyper-V 保留端口范围导致 EACCES）。

## 测试 / 验证方法

- Rust 后端类型检查：`cd src-tauri && cargo check`
- 前端类型检查与构建：`npm run build`
- 下载链路手工验证（与应用内部完全相同的参数）：

```powershell
src-tauri\binaries\yt-dlp.exe -f "bv*[height<=1080][vcodec^=avc1]+ba[ext=m4a]/b[height<=1080]" `
  --merge-output-format mp4 --no-playlist -c --newline --progress `
  --progress-template "%(progress)j" --no-simulate `
  --print "after_move:FILEPATH::%(filepath)s" `
  --ffmpeg-location src-tauri\binaries\ffmpeg.exe -o "%(title)s.%(ext)s" -- <视频URL>
```

- 应用内验收路径：
  1. 启动后顶栏显示 yt-dlp / ffmpeg 版本号（Phase 0 验收）
  2. 粘贴单视频链接 → 解析 → 选格式 → 下载 → 进度条 → 完成后"打开所在文件夹"（Phase 1）
  3. 粘贴播放列表链接 → 勾选 → 批量下载；下载中退出重开 App，任务恢复并续传（Phase 2）
  4. 设置中修改下载目录/并发数/命名模板；点击"更新下载引擎"后版本号变化（Phase 3）

## 打包

```bash
npm run tauri build     # 安装包
npm run build:win       # Windows：安装包 + 便携版 zip 一次生成
```

- Windows 产物：
  - `src-tauri/target/release/bundle/nsis/*.exe`（NSIS 安装包）
  - `src-tauri/target/release/bundle/portable/*.zip`（便携版免安装，解压即用；由 `scripts/make-portable.ps1` 生成，可单独在 tauri build 之后运行）
- macOS 产物：`src-tauri/target/release/bundle/dmg/*.dmg`

> 便携版依赖系统的 WebView2 运行时（Win10/11 一般自带）；设置与下载队列仍保存在 `%APPDATA%\com.amose.tubegrab\`。

> **打包报 `Access is denied. (os error 5)`**：上一次生成的安装包正被杀软实时扫描/索引占用，makensis 覆盖失败。删除 `src-tauri/target/release/bundle/nsis/` 后重试即可：
> `Remove-Item src-tauri\target\release\bundle\nsis -Recurse -Force`

### GitHub Actions

推送 `v*` 标签（或手动 workflow_dispatch）触发 `.github/workflows/build.yml`，自动构建：

- Windows x64（NSIS）
- macOS Apple Silicon（DMG）
- macOS Intel（DMG）

### macOS 未签名说明

无 Apple 开发者账号时应用未签名/未公证，首次打开方式：

1. 右键 App → 打开 → 再点"打开"；或
2. 终端执行 `xattr -cr /Applications/TubeGrab.app` 清除隔离属性。

## 关键实现说明

- **引擎自更新**：安装目录通常不可写，因此首次启动时把捆绑的 yt-dlp 复制到应用数据目录（Windows：`%APPDATA%/com.amose.tubegrab/engine/`），之后一律从该副本运行；"更新下载引擎"执行官方 `-U` 自更新覆盖该副本。YouTube 改版导致解析失败时，更新引擎即可修复，无需发新版 App。
- **进度协议**：`--newline --progress-template "%(progress)j"` 逐行输出 JSON，Rust 解析后经 Tauri event `task-updated` 推送前端；最终文件路径通过 `--print "after_move:FILEPATH::%(filepath)s"` 获取。
- **断点续传**：始终带 `-c`，`.part` 文件保留；未完成队列持久化在应用数据目录 `queue.json`，重启后自动恢复下载。
- **格式策略**：视频强制 h264（`vcodec^=avc1`）+ m4a 音轨合并为 MP4，保证播放器兼容性。

## v1 明确不支持

需登录内容（会员/私享/年龄限制）、直播录制、内置播放器。

## 免责声明

本工具仅供个人学习与自用（下载自己有权访问的内容）。请遵守 YouTube 服务条款及当地版权法律，勿用于商业或侵权用途。
