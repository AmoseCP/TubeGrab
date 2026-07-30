# TubeGrab — 跨平台 YouTube 视频/音频下载工具开发计划

> 目标:Windows + macOS 桌面应用,无需登录 YouTube,粘贴链接即可下载视频(MP4)或音频(MP3/M4A)。
> 架构核心:GUI 外壳 + yt-dlp 引擎 + ffmpeg 后处理。**不自己实现解析逻辑**。
> 开发方式:由 AI(Claude Code)从零开发,人工负责验收和真机测试。
> 依据 `TubeGrab开发.md` 的完整需求制定。遵循 CLAUDE.md 准则：每一步有明确的验证标准，只实现需求中列出的功能，不做投机性设计。每次开始新的开发工作前务必阅读CLAUDE.md。相关测试，运行，部署方法定入readme，已经做过的事，避免忘记写入memory。

---

## 一、架构总览

```
┌─────────────────────────────────────┐
│  Tauri App (React 前端 + Rust 后端)   │
│                                     │
│  React UI ──invoke──► Rust Commands │
│                          │          │
│                    spawn 子进程       │
│                          ▼          │
│              yt-dlp (捆绑二进制)      │
│                          │          │
│                    调用 ffmpeg       │
│                  (合并/转码,捆绑)     │
└─────────────────────────────────────┘
```

| 组件 | 选择 | 说明 |
|------|------|------|
| 框架 | Tauri 2 | 打包 5~10MB,Win/Mac 双端,Rust 后端 |
| 前端 | React + Tailwind | 复用你熟悉的技术栈 |
| 下载引擎 | yt-dlp(捆绑独立二进制) | 社区维护解析逻辑,支持进度输出 JSON |
| 后处理 | ffmpeg(捆绑) | 音视频流合并、MP3 转码 |
| 引擎自更新 | yt-dlp 的 `-U` 或下载最新 release | YouTube 一变,用户端一键更新引擎即可,无需发新版 App |

**为什么捆绑二进制而不是让用户自己装:** 目标用户不是开发者,双击即用是硬需求。yt-dlp 官方提供 `yt-dlp.exe`(Win)和 `yt-dlp_macos`(Mac)独立二进制;ffmpeg 用 BtbN/evermeet 的静态构建。

---

## 二、功能规划

### v1 核心功能
1. **粘贴链接 → 解析**:调用 `yt-dlp -J <url>` 获取 JSON 元数据(标题、封面、时长、可用格式列表)
2. **格式选择**:
   - 视频:1080p / 720p / 480p(MP4,自动选 h264 保证兼容性)
   - 音频:MP3(转码)/ M4A(原始流,更快无损)
3. **下载队列**:多任务排队,单任务实时进度条(速度、剩余时间、百分比)
4. **播放列表支持**:检测到 playlist 链接时列出所有视频,可勾选批量下载
5. **基础设置**:下载目录、同时下载数、文件命名模板
6. **失败重试** + 断点续传(yt-dlp 原生支持 `-c`)

### v1 明确不做
- ❌ 需要登录的内容(会员视频、私享视频、年龄限制内容)
- ❌ 直播录制
- ❌ 内置播放器

### v2 展望
- 字幕下载(yt-dlp 原生支持)
- 剪辑片段下载(`--download-sections`)
- 更多站点(yt-dlp 本身支持上千个站点,UI 上放开即可)

---

## 三、关键技术点

### 1. 调用 yt-dlp 并解析进度
```
yt-dlp --newline --progress-template "%(progress)j" -f <format_id> -o <template> <url>
```
Rust 端用 `tokio::process::Command` spawn 子进程,逐行读 stdout,解析 JSON 进度,通过 Tauri event 推送给前端。

### 2. 格式选择策略(重要,避免下载后打不开)
- 1080p MP4:`-f "bv*[height<=1080][vcodec^=avc1]+ba[ext=m4a]/b[height<=1080]" --merge-output-format mp4`
- MP3:`-f "ba" -x --audio-format mp3 --audio-quality 0`
- YouTube 高清视频是音视频分离的(DASH),必须 ffmpeg 合并——这就是为什么要捆绑 ffmpeg

### 3. 二进制的平台差异处理
- Tauri 的 `sidecar` 机制按平台捆绑对应二进制(`yt-dlp-x86_64-pc-windows-msvc.exe` / `yt-dlp-aarch64-apple-darwin` 等)
- macOS 注意:二进制需要 `chmod +x`,且要处理 Gatekeeper 隔离属性
- Mac 要同时支持 Intel 和 Apple Silicon(universal 构建或分别打包)

### 4. yt-dlp 失效应对(必做)
- 设置页放"更新下载引擎"按钮:从 GitHub releases 拉最新 yt-dlp 覆盖
- 解析失败时提示"引擎可能过期,请点击更新"——这是此类工具最常见的故障和修复路径

---

## 四、分阶段开发计划

### Phase 0:骨架(0.5 天)
- `pnpm create tauri-app`,React + TypeScript 模板
- 配置 sidecar 捆绑 yt-dlp 和 ffmpeg(先只配当前开发平台)
- **验收:App 启动,能调用 `yt-dlp --version` 并在 UI 显示版本号**

### Phase 1:解析与单任务下载(2 天)
- 粘贴链接 → 调用 `-J` 解析 → 展示标题/封面/时长/格式选项
- 选格式 → 下载 → 实时进度条 → 完成后"打开所在文件夹"
- 错误处理:无效链接、网络失败、引擎报错的友好提示
- **验收:完整走通一个 1080p 视频和一个 MP3 下载**

### Phase 2:队列与播放列表(2 天)
- 下载队列(并发数可配,默认 2),任务状态机:排队/下载中/合并中/完成/失败
- 播放列表解析(`--flat-playlist -J`)+ 勾选批量加入队列
- 断点续传与失败重试
- **验收:20 个视频的播放列表批量下载稳定完成;中途退出 App 重开可续传**

### Phase 3:设置与引擎更新(1 天)
- 设置页:下载目录、并发数、命名模板、默认格式
- yt-dlp 一键更新(GitHub release 下载 + 校验)
- **验收:更新引擎后版本号变化,下载功能正常**

### Phase 4:双平台打包(1~2 天)
- Windows:NSIS 安装包;macOS:DMG(Intel + Apple Silicon)
- macOS 签名与公证(无开发者账号则文档说明如何绕过 Gatekeeper:右键打开)
- GitHub Actions 双平台自动构建
- **验收:两台干净的机器(无开发环境)安装后直接可用**

**总计约 7~9 个工作日**

---

## 五、给 AI 的开发提示词模板

**Phase 0:**
```
用 Tauri 2 + React + TypeScript 创建项目 TubeGrab。
配置 Tauri sidecar 捆绑 yt-dlp 二进制(先配 macOS aarch64,目录 src-tauri/binaries/)。
实现一个 Rust command `get_engine_version`,spawn yt-dlp --version 并返回输出。
前端启动时调用并显示版本号。给出完整配置文件和代码。
```

**Phase 1 关键提示词:**
```
实现视频解析与下载:
1. Rust command `parse_url(url)`:spawn yt-dlp -J --no-playlist <url>,
   解析 JSON 返回 {title, thumbnail, duration, formats[]}
2. Rust command `start_download(url, format_selector, output_dir)`:
   spawn yt-dlp 带 --newline --progress-template "%(progress)j",
   逐行解析 stdout,通过 Tauri event "download-progress" 推送 {percent, speed, eta}
3. React 端:输入框+解析按钮 → 展示卡片(封面/标题/格式下拉) → 下载按钮 → 进度条
格式选择器用:[粘贴上文第三节的 -f 参数]
注意:所有子进程错误要捕获并返回结构化错误信息,不能让 App 崩溃。
```

**开发纪律(写进 CLAUDE.md):**
- 永远不要自己实现 YouTube URL 解析/解密,一切通过 yt-dlp
- 子进程调用必须异步(tokio),绝不阻塞主线程
- 用户输入的 URL 必须校验后再传给子进程(防止命令注入,虽然用参数数组传递本身就安全,但仍要校验)
- 下载路径处理用平台无关 API,注意 Windows 路径反斜杠和 mac 权限

---

## 六、风险与注意事项

| 风险 | 对策 |
|------|------|
| YouTube 更新导致解析失效 | 引擎一键更新机制(核心保命功能) |
| YouTube ToS / 版权问题 | 个人工具自用;若公开分发,加免责声明,不做商业化;不提供绕过登录/年龄限制的功能 |
| 杀毒软件误报(下载器常被误报) | 代码签名;不做混淆 |
| YouTube IP 限速/封锁 | v1 不处理;提示用户降低并发 |
| ffmpeg/yt-dlp 二进制体积(~100MB) | 可接受;或首次启动时下载(增加复杂度,v1 不建议) |
