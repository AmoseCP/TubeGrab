# TubeGrab 开发记录（已完成事项，避免重复劳动）

- 2026-07-30 由 Claude Code 按 `TubeGrab开发.md` + `DEV_PLAN.md` 完成 v1 全量开发（Phase 0~4）。
- 技术决策（详见 DEV_PLAN.md 第 0 节）：
  - 二进制用 `bundle.resources` 捆绑（非 sidecar），因为引擎自更新需要覆盖 yt-dlp 自身；首启复制到 appdata/engine/，之后从副本运行。
  - 引擎更新用 yt-dlp 官方 `-U` 自更新（自带校验），不自己下载 release。
  - Vite 端口 **15420**（不是默认 1420）：本机 1420 落在 Hyper-V 保留端口范围，报 EACCES。
  - 子进程统一经 `engine::new_command`（Windows CREATE_NO_WINDOW + PYTHONIOENCODING=utf-8）。
  - 最终文件路径经 `--print "after_move:FILEPATH::%(filepath)s"` 获取（实测 mp4 合并与 mp3 转码都返回正确最终路径）。
- 已验证：cargo check / npm run build 通过；与产品完全相同参数实测 480p MP4 下载与 MP3 转码成功；tauri dev 冒烟通过（appdata 引擎副本自动生成，证明前端 invoke 链路正常）。
- 本机 Rust 需 ≥1.88（已从 1.84 升级到 1.97.1，旧版因 edition2024 无法编译 Tauri 2 依赖）。
- 二进制不入库（src-tauri/.gitignore 排除 /binaries/），重建用 scripts/fetch-binaries.ps1（Win）/ .sh（Mac）。
- 2026-07-30 增加"关于"对话框（AboutModal.tsx）：软件版本（getVersion 动态读取）、引擎版本、开发者 Telegram @Dingjin2025（openUrl 打开，capabilities 已加 opener:allow-open-url 允许 https://t.me/*）、免责声明。
- 坑：`tauri build` 偶发 `Access is denied (os error 5)` —— 旧安装包被杀软实时扫描占用，makensis 覆盖失败；删 bundle/nsis 目录重试即可（已写入 README 故障排查）。
- 2026-07-30 修复：`list=RD*`/`start_radio` 是 YouTube Mix 电台（自动生成的无限推荐流），不再默认按播放列表解析（App.tsx isMixRadio），默认只下当前视频，勾选框可手动切回列表模式。
- 2026-07-30 解析卡片改为 FormatPicker 面板：视频/音频切换，列出源实际可用分辨率（码率+预估大小，parser.rs 返回 videoOptions/audio）；格式串支持任意 `mp4-<高度>`（download.rs format_args 动态生成）。
- 坑：高分辨率（4K/1440p）通常无 h264 只有 vp9/av1，格式选择器必须用 `height=H` 精确匹配并逐级回退，否则 `height<=H + vcodec^=avc1` 会静默降到 1080p（已用 --simulate 验证修复）。
- 2026-07-30 新增 Windows 便携版：scripts/make-portable.ps1（exe + binaries/ + 说明.txt 打 zip），`npm run build:win` 一次出安装包+便携包，CI 的 Windows job 也产出。已实测解压到任意目录可运行（资源按 exe 同级解析）。
- 坑：.ps1 含中文必须存成 UTF-8 **带 BOM**，否则 PowerShell 5.1 按 ANSI 读取会破坏 here-string 终止符（报 "missing the terminator"）。
- 尚未做（v2 展望）：字幕下载、片段下载、多站点放开。
