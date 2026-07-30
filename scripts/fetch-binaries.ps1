# Downloads yt-dlp and ffmpeg (Windows x64) into src-tauri/binaries/.
# Run from repo root:  powershell -ExecutionPolicy Bypass -File scripts/fetch-binaries.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$binDir = Join-Path $root 'src-tauri\binaries'
New-Item -ItemType Directory -Force $binDir | Out-Null

$ytDlp = Join-Path $binDir 'yt-dlp.exe'
Write-Host 'Downloading yt-dlp.exe ...'
Invoke-WebRequest -Uri 'https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe' -OutFile $ytDlp

Write-Host 'Downloading ffmpeg (BtbN static build) ...'
$zip = Join-Path $env:TEMP 'ffmpeg-tubegrab.zip'
Invoke-WebRequest -Uri 'https://github.com/BtbN/FFmpeg-Builds/releases/latest/download/ffmpeg-master-latest-win64-gpl.zip' -OutFile $zip
$extractDir = Join-Path $env:TEMP 'ffmpeg-tubegrab'
if (Test-Path $extractDir) { Remove-Item $extractDir -Recurse -Force }
Expand-Archive $zip -DestinationPath $extractDir
$ffmpegExe = Get-ChildItem $extractDir -Recurse -Filter 'ffmpeg.exe' | Select-Object -First 1
Copy-Item $ffmpegExe.FullName (Join-Path $binDir 'ffmpeg.exe') -Force
Remove-Item $zip -Force
Remove-Item $extractDir -Recurse -Force

Write-Host 'Done:'
Get-ChildItem $binDir | Format-Table Name, Length
