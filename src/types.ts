export interface EngineInfo {
  ytdlp_version: string;
  ffmpeg_version: string;
}

export interface ApiError {
  kind: "invalid_url" | "network" | "engine" | "unsupported" | "internal";
  message: string;
}

export interface VideoOption {
  height: number;
  /** 总码率 kbps */
  tbr: number | null;
  /** 预估大小 bytes（视频+音频） */
  filesize: number | null;
}

export interface AudioInfo {
  /** 最佳音频流码率 kbps */
  abr: number | null;
  filesize: number | null;
}

export interface VideoInfo {
  url: string;
  title: string;
  thumbnail: string | null;
  duration: number | null;
  uploader: string | null;
  /** 源里实际可用的分辨率档位（降序） */
  videoOptions: VideoOption[];
  audio: AudioInfo;
}

export interface PlaylistEntry {
  url: string;
  title: string;
  duration: number | null;
  thumbnail: string | null;
}

export interface PlaylistInfo {
  title: string;
  entries: PlaylistEntry[];
}

export type TaskStatus =
  | "queued"
  | "downloading"
  | "merging"
  | "done"
  | "failed"
  | "canceled";

export interface Task {
  id: number;
  url: string;
  title: string;
  thumbnail: string | null;
  format: string;
  status: TaskStatus;
  percent: number;
  speed: number | null;
  eta: number | null;
  error: string | null;
  filepath: string | null;
}

export interface Settings {
  downloadDir: string;
  concurrency: number;
  filenameTemplate: string;
  defaultFormat: string;
}

export interface FormatPreset {
  value: string;
  label: string;
  height?: number;
}

export const FORMATS: FormatPreset[] = [
  { value: "mp4-1080", label: "视频 MP4 · 1080p", height: 1080 },
  { value: "mp4-720", label: "视频 MP4 · 720p", height: 720 },
  { value: "mp4-480", label: "视频 MP4 · 480p", height: 480 },
  { value: "mp3", label: "音频 MP3（转码）" },
  { value: "m4a", label: "音频 M4A（原始流，更快）" },
];

/** 根据源的最大清晰度决定各视频档位是否可选：
 *  高于源清晰度的档位与更低档位结果相同，禁用之；
 *  但保留能覆盖源清晰度的最小档位。 */
export function isFormatEnabled(preset: FormatPreset, maxHeight: number | null): boolean {
  if (preset.height === undefined) return true; // 音频总是可选
  if (maxHeight === null || maxHeight <= 0) return true;
  if (preset.height <= maxHeight) return true;
  const videoHeights = FORMATS.filter((f) => f.height !== undefined).map((f) => f.height!);
  const covering = Math.min(...videoHeights.filter((h) => h >= maxHeight));
  return preset.height === covering;
}

export function formatLabel(value: string): string {
  const h = value.match(/^mp4-(\d+)$/)?.[1];
  if (h) return `视频 MP4 · ${h}p`;
  return FORMATS.find((f) => f.value === value)?.label ?? value;
}

export function formatSize(bytes: number | null): string {
  if (bytes === null || bytes <= 0) return "";
  const units = ["B", "KB", "MB", "GB"];
  let v = bytes;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `≈${v.toFixed(v >= 100 ? 0 : 1)} ${units[i]}`;
}

export function formatBitrate(kbps: number | null): string {
  if (kbps === null || kbps <= 0) return "";
  return kbps >= 1000 ? `${(kbps / 1000).toFixed(1)} Mbps` : `${Math.round(kbps)} kbps`;
}

export function formatDuration(secs: number | null): string {
  if (secs === null || !isFinite(secs)) return "--:--";
  const s = Math.round(secs);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const mm = String(m).padStart(2, "0");
  const ss = String(sec).padStart(2, "0");
  return h > 0 ? `${h}:${mm}:${ss}` : `${m}:${ss}`;
}

export function formatSpeed(bytesPerSec: number | null): string {
  if (bytesPerSec === null || bytesPerSec <= 0) return "";
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  let v = bytesPerSec;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(1)} ${units[i]}`;
}

export function formatEta(secs: number | null): string {
  if (secs === null || !isFinite(secs) || secs <= 0) return "";
  return `剩余 ${formatDuration(secs)}`;
}
