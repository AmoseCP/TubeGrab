import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, toApiError } from "../api";
import { EngineInfo, FORMATS, Settings } from "../types";

interface Props {
  settings: Settings;
  engine: EngineInfo | null;
  onClose: () => void;
  onSaved: (s: Settings) => void;
  onEngineUpdated: () => void;
}

export default function SettingsModal({ settings, engine, onClose, onSaved, onEngineUpdated }: Props) {
  const [draft, setDraft] = useState<Settings>({ ...settings });
  const [updating, setUpdating] = useState(false);
  const [updateMsg, setUpdateMsg] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  async function pickDir() {
    const dir = await open({ directory: true, defaultPath: draft.downloadDir });
    if (typeof dir === "string") setDraft({ ...draft, downloadDir: dir });
  }

  async function save() {
    setSaving(true);
    try {
      const cleaned: Settings = {
        ...draft,
        concurrency: Math.min(8, Math.max(1, Math.round(draft.concurrency) || 1)),
        filenameTemplate: draft.filenameTemplate.trim() || "%(title)s.%(ext)s",
      };
      await api.saveSettings(cleaned);
      onSaved(cleaned);
      onClose();
    } catch (e) {
      setUpdateMsg(toApiError(e).message);
    } finally {
      setSaving(false);
    }
  }

  async function updateEngine() {
    setUpdating(true);
    setUpdateMsg(null);
    try {
      const version = await api.updateEngine();
      setUpdateMsg(`引擎已是最新/更新完成，当前版本：${version}`);
      onEngineUpdated();
    } catch (e) {
      setUpdateMsg(toApiError(e).message);
    } finally {
      setUpdating(false);
    }
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={onClose}>
      <div
        className="w-[520px] max-w-[92vw] rounded-2xl bg-white p-5 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-base font-semibold text-zinc-800">设置</h2>
          <button className="text-zinc-400 hover:text-zinc-600" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="flex flex-col gap-4 text-sm">
          <label className="flex flex-col gap-1.5">
            <span className="text-zinc-600">下载目录</span>
            <div className="flex gap-2">
              <input
                className="flex-1 rounded-lg border border-zinc-300 px-3 py-2 focus:border-blue-500 focus:outline-none"
                value={draft.downloadDir}
                onChange={(e) => setDraft({ ...draft, downloadDir: e.target.value })}
              />
              <button
                className="rounded-lg border border-zinc-300 px-3 py-2 hover:bg-zinc-50"
                onClick={pickDir}
              >
                浏览…
              </button>
            </div>
          </label>

          <div className="flex gap-4">
            <label className="flex flex-1 flex-col gap-1.5">
              <span className="text-zinc-600">同时下载数（1-8）</span>
              <input
                type="number"
                min={1}
                max={8}
                className="rounded-lg border border-zinc-300 px-3 py-2 focus:border-blue-500 focus:outline-none"
                value={draft.concurrency}
                onChange={(e) => setDraft({ ...draft, concurrency: Number(e.target.value) })}
              />
            </label>
            <label className="flex flex-1 flex-col gap-1.5">
              <span className="text-zinc-600">默认格式</span>
              <select
                className="rounded-lg border border-zinc-300 bg-white px-3 py-2 focus:border-blue-500 focus:outline-none"
                value={draft.defaultFormat}
                onChange={(e) => setDraft({ ...draft, defaultFormat: e.target.value })}
              >
                {FORMATS.map((f) => (
                  <option key={f.value} value={f.value}>
                    {f.label}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <label className="flex flex-col gap-1.5">
            <span className="text-zinc-600">
              文件命名模板（yt-dlp 模板语法，如 %(title)s.%(ext)s）
            </span>
            <input
              className="rounded-lg border border-zinc-300 px-3 py-2 font-mono text-xs focus:border-blue-500 focus:outline-none"
              value={draft.filenameTemplate}
              onChange={(e) => setDraft({ ...draft, filenameTemplate: e.target.value })}
            />
          </label>

          <div className="rounded-xl bg-zinc-50 p-3">
            <div className="flex items-center justify-between">
              <div className="text-zinc-600">
                下载引擎 yt-dlp
                <span className="ml-2 font-mono text-xs text-zinc-500">
                  {engine?.ytdlp_version ?? "未知"}
                </span>
              </div>
              <button
                className="rounded-lg bg-blue-600 px-3 py-1.5 text-xs text-white hover:bg-blue-700 disabled:opacity-50"
                disabled={updating}
                onClick={updateEngine}
              >
                {updating ? "更新中…" : "更新下载引擎"}
              </button>
            </div>
            <p className="mt-1.5 text-xs text-zinc-400">
              解析或下载频繁失败时，通常是 YouTube 改版导致引擎过期，点击更新即可修复。
            </p>
            {updateMsg && <p className="mt-1.5 text-xs text-zinc-600">{updateMsg}</p>}
          </div>
        </div>

        <div className="mt-5 flex justify-end gap-2">
          <button
            className="rounded-lg border border-zinc-300 px-4 py-2 text-sm hover:bg-zinc-50"
            onClick={onClose}
          >
            取消
          </button>
          <button
            className="rounded-lg bg-blue-600 px-4 py-2 text-sm text-white hover:bg-blue-700 disabled:opacity-50"
            disabled={saving}
            onClick={save}
          >
            保存
          </button>
        </div>
      </div>
    </div>
  );
}
