# Assembles a portable (no-install) Windows package from the release build.
# Run after `npm run tauri build`, or via `npm run build:win`.
# Output: src-tauri/target/release/bundle/portable/TubeGrab_<version>_x64-portable.zip
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$conf = Get-Content (Join-Path $root 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json
$version = $conf.version
$release = Join-Path $root 'src-tauri\target\release'
$exe = Join-Path $release 'tubegrab.exe'
if (-not (Test-Path $exe)) { throw "tubegrab.exe not found. Run 'npm run tauri build' first." }

$stageRoot = Join-Path $release 'portable-stage'
$stage = Join-Path $stageRoot 'TubeGrab'
if (Test-Path $stageRoot) { Remove-Item $stageRoot -Recurse -Force }
New-Item -ItemType Directory -Force (Join-Path $stage 'binaries') | Out-Null

Copy-Item $exe (Join-Path $stage 'TubeGrab.exe')
Copy-Item (Join-Path $release 'binaries\yt-dlp.exe') (Join-Path $stage 'binaries\')
Copy-Item (Join-Path $release 'binaries\ffmpeg.exe') (Join-Path $stage 'binaries\')

@"
TubeGrab $version 便携版（免安装）
================================

双击 TubeGrab.exe 即可运行，无需安装。

- 依赖 Microsoft Edge WebView2 运行时（Windows 10/11 一般已自带；
  若启动报错，从 https://developer.microsoft.com/microsoft-edge/webview2/ 安装）
- 设置、下载队列与引擎更新数据保存在 %APPDATA%\com.amose.tubegrab\
- binaries 目录必须与 TubeGrab.exe 保持同级，请整个文件夹一起移动
"@ | Out-File (Join-Path $stage '说明.txt') -Encoding utf8

$outDir = Join-Path $release 'bundle\portable'
New-Item -ItemType Directory -Force $outDir | Out-Null
$zip = Join-Path $outDir "TubeGrab_${version}_x64-portable.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path $stage -DestinationPath $zip
Remove-Item $stageRoot -Recurse -Force
Write-Host "Portable package: $zip"
