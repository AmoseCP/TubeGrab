import { invoke } from "@tauri-apps/api/core";
import type {
  ApiError,
  AppUpdateInfo,
  EngineInfo,
  PlaylistInfo,
  Settings,
  Task,
  VideoInfo,
} from "./types";

/** invoke 失败时抛出的是后端序列化的 ApiError；此处兜底非结构化错误。 */
export function toApiError(e: unknown): ApiError {
  if (e && typeof e === "object" && "kind" in e && "message" in e) {
    return e as ApiError;
  }
  return { kind: "internal", message: String(e) };
}

export const api = {
  getEngineVersion: () => invoke<EngineInfo>("get_engine_version"),
  updateEngine: () => invoke<string>("update_engine"),
  parseUrl: (url: string) => invoke<VideoInfo>("parse_url", { url }),
  parsePlaylist: (url: string) => invoke<PlaylistInfo>("parse_playlist", { url }),
  addTasks: (items: { url: string; title: string; thumbnail: string | null; format: string }[]) =>
    invoke<Task[]>("add_tasks", { items }),
  getTasks: () => invoke<Task[]>("get_tasks"),
  cancelTask: (id: number) => invoke<void>("cancel_task", { id }),
  retryTask: (id: number) => invoke<void>("retry_task", { id }),
  removeTask: (id: number) => invoke<void>("remove_task", { id }),
  openInFolder: (path: string) => invoke<void>("open_in_folder", { path }),
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<void>("save_settings", { settings }),
  checkAppUpdate: () => invoke<AppUpdateInfo>("check_app_update"),
  installAppUpdate: (url: string) => invoke<void>("install_app_update", { url }),
};
