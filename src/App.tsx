import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, toApiError } from "./api";
import AboutModal from "./components/AboutModal";
import FormatPicker from "./components/FormatPicker";
import FormatSelect from "./components/FormatSelect";
import SettingsModal from "./components/SettingsModal";
import TaskList from "./components/TaskList";
import {
  ApiError,
  AppUpdateInfo,
  EngineInfo,
  formatDuration,
  PlaylistInfo,
  Settings,
  Task,
  VideoInfo,
} from "./types";

type ParseResult =
  | { kind: "video"; info: VideoInfo }
  | { kind: "playlist"; info: PlaylistInfo };

/** 按用户默认格式在实际可用档位中选最接近的：
 *  音频直接用；视频选不超过默认清晰度的最高档，没有则取最低档。 */
function pickDefaultFormat(info: VideoInfo, preferred: string): string {
  if (preferred === "mp3" || preferred === "m4a") return preferred;
  const heights = info.videoOptions.map((o) => o.height);
  if (heights.length === 0) return "m4a";
  const want = Number(preferred.match(/^mp4-(\d+)$/)?.[1] ?? 1080);
  const fit = heights.filter((h) => h <= want);
  return `mp4-${fit.length ? Math.max(...fit) : Math.min(...heights)}`;
}

export default function App() {
  const [engine, setEngine] = useState<EngineInfo | null>(null);
  const [engineError, setEngineError] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [showAbout, setShowAbout] = useState(false);

  const [url, setUrl] = useState("");
  const [asPlaylist, setAsPlaylist] = useState(false);
  const [parsing, setParsing] = useState(false);
  const [parseError, setParseError] = useState<ApiError | null>(null);
  const [result, setResult] = useState<ParseResult | null>(null);
  const [format, setFormat] = useState("mp4-1080");
  const [checked, setChecked] = useState<Set<number>>(new Set());

  const [tasks, setTasks] = useState<Task[]>([]);
  const urlRef = useRef<HTMLInputElement>(null);

  const [appUpdate, setAppUpdate] = useState<AppUpdateInfo | null>(null);
  const [updateBusy, setUpdateBusy] = useState(false);
  const [updatePct, setUpdatePct] = useState(0);
  const [updateError, setUpdateError] = useState<string | null>(null);

  const refreshEngine = useCallback(() => {
    api
      .getEngineVersion()
      .then((info) => {
        setEngine(info);
        setEngineError(null);
      })
      .catch((e) => setEngineError(toApiError(e).message));
  }, []);

  useEffect(() => {
    refreshEngine();
    api.getSettings().then((s) => {
      setSettings(s);
      setFormat(s.defaultFormat);
    });
    api.getTasks().then(setTasks);
    // 启动时静默检查应用更新，失败不打扰用户（下次启动会再试）
    api
      .checkAppUpdate()
      .then((u) => {
        if (u.available) setAppUpdate(u);
      })
      .catch(() => {});
    const unlistenUpdate = listen<{ downloaded: number; total: number }>(
      "app-update-progress",
      (event) => {
        const { downloaded, total } = event.payload;
        setUpdatePct(total > 0 ? Math.round((downloaded / total) * 100) : 0);
      },
    );
    const unlisten = listen<Task>("task-updated", (event) => {
      setTasks((prev) => {
        const t = event.payload;
        const idx = prev.findIndex((x) => x.id === t.id);
        if (idx === -1) return [...prev, t];
        const next = [...prev];
        next[idx] = t;
        return next;
      });
    });
    return () => {
      unlisten.then((fn) => fn());
      unlistenUpdate.then((fn) => fn());
    };
  }, [refreshEngine]);

  async function doAppUpdate() {
    if (!appUpdate || updateBusy) return;
    setUpdateError(null);
    if (appUpdate.canAutoInstall && appUpdate.assetUrl) {
      setUpdateBusy(true);
      setUpdatePct(0);
      try {
        // 下载完成后会自动启动安装程序并退出本应用
        await api.installAppUpdate(appUpdate.assetUrl);
      } catch (e) {
        setUpdateBusy(false);
        setUpdateError(toApiError(e).message);
      }
    } else {
      openUrl(appUpdate.pageUrl).catch(() => {});
    }
  }

  // 输入变化时自动识别播放列表链接。
  // list=RD*/start_radio 是 YouTube 自动生成的 Mix 电台（无限推荐流），
  // 不是用户创建的播放列表，默认只解析当前视频。
  function isMixRadio(v: string): boolean {
    const list = v.match(/[?&]list=([^&]+)/)?.[1];
    return !list || list.startsWith("RD") || v.includes("start_radio");
  }

  function onUrlChange(v: string) {
    setUrl(v);
    setAsPlaylist(v.includes("list=") && !isMixRadio(v));
  }

  async function parse() {
    const target = url.trim();
    if (!target || parsing) return;
    setParsing(true);
    setParseError(null);
    setResult(null);
    try {
      const preferred = settings?.defaultFormat ?? "mp4-1080";
      if (asPlaylist) {
        const info = await api.parsePlaylist(target);
        setResult({ kind: "playlist", info });
        setChecked(new Set(info.entries.map((_, i) => i)));
        setFormat(preferred); // 播放列表用固定预设下拉
      } else {
        const info = await api.parseUrl(target);
        setResult({ kind: "video", info });
        setFormat(pickDefaultFormat(info, preferred));
      }
    } catch (e) {
      setParseError(toApiError(e));
    } finally {
      setParsing(false);
    }
  }

  async function download() {
    if (!result) return;
    const items =
      result.kind === "video"
        ? [
            {
              url: result.info.url,
              title: result.info.title,
              thumbnail: result.info.thumbnail,
              format,
            },
          ]
        : result.info.entries
            .filter((_, i) => checked.has(i))
            .map((e) => ({
              url: e.url,
              title: e.title,
              thumbnail: e.thumbnail,
              format,
            }));
    if (items.length === 0) return;
    try {
      await api.addTasks(items);
      setResult(null);
      setUrl("");
      urlRef.current?.focus();
    } catch (e) {
      setParseError(toApiError(e));
    }
  }

  function toggleChecked(i: number) {
    setChecked((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });
  }

  return (
    <div className="min-h-screen bg-zinc-50 text-zinc-900">
      <header className="sticky top-0 z-10 border-b border-zinc-200 bg-white/90 px-5 py-3 backdrop-blur">
        <div className="mx-auto flex max-w-3xl items-center justify-between">
          <div className="flex items-baseline gap-3">
            <h1 className="text-lg font-bold">TubeGrab</h1>
            <span className="font-mono text-xs text-zinc-400">
              {engine
                ? `yt-dlp ${engine.ytdlp_version} · ffmpeg ${engine.ffmpeg_version}`
                : engineError
                  ? "引擎不可用"
                  : "引擎加载中…"}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              className="rounded-lg border border-zinc-300 px-3 py-1.5 text-sm text-zinc-700 hover:bg-zinc-100"
              onClick={() => setShowAbout(true)}
            >
              关于
            </button>
            <button
              className="rounded-lg border border-zinc-300 px-3 py-1.5 text-sm text-zinc-700 hover:bg-zinc-100"
              onClick={() => setShowSettings(true)}
            >
              设置
            </button>
          </div>
        </div>
      </header>

      <main className="mx-auto flex max-w-3xl flex-col gap-4 px-5 py-5">
        {appUpdate && (
          <div className="flex items-center justify-between gap-3 rounded-xl bg-blue-50 p-3 text-sm text-blue-800">
            <div className="min-w-0">
              <span>
                发现新版本 {appUpdate.latest}（当前 v{appUpdate.current}）
              </span>
              {updateError && (
                <div className="mt-1 text-xs text-red-600">{updateError}</div>
              )}
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <button
                className="rounded-lg bg-blue-600 px-4 py-1.5 text-xs font-medium text-white hover:bg-blue-700 disabled:opacity-60"
                disabled={updateBusy}
                onClick={doAppUpdate}
              >
                {updateBusy
                  ? `下载中 ${updatePct}%`
                  : appUpdate.canAutoInstall
                    ? "立即更新"
                    : "前往下载"}
              </button>
              {!updateBusy && (
                <button
                  className="text-blue-400 hover:text-blue-600"
                  title="本次忽略"
                  onClick={() => setAppUpdate(null)}
                >
                  ✕
                </button>
              )}
            </div>
          </div>
        )}

        {engineError && (
          <div className="rounded-xl bg-red-50 p-3 text-sm text-red-700">
            下载引擎不可用：{engineError}
          </div>
        )}

        {/* 输入区 */}
        <div className="flex gap-2">
          <input
            ref={urlRef}
            className="flex-1 rounded-xl border border-zinc-300 bg-white px-4 py-2.5 text-sm shadow-sm focus:border-blue-500 focus:outline-none"
            placeholder="粘贴 YouTube 视频或播放列表链接…"
            value={url}
            onChange={(e) => onUrlChange(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && parse()}
          />
          <button
            className="rounded-xl bg-blue-600 px-5 py-2.5 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
            disabled={parsing || !url.trim()}
            onClick={parse}
          >
            {parsing ? "解析中…" : "解析"}
          </button>
        </div>
        {url.includes("list=") && (
          <label className="-mt-2 flex items-center gap-2 text-xs text-zinc-500">
            <input
              type="checkbox"
              checked={asPlaylist}
              onChange={(e) => setAsPlaylist(e.target.checked)}
            />
            作为播放列表解析（取消则只下载当前视频）
          </label>
        )}

        {/* 解析错误 */}
        {parseError && (
          <div className="rounded-xl bg-red-50 p-3 text-sm text-red-700">
            <div className="whitespace-pre-wrap">{parseError.message}</div>
            {parseError.kind === "engine" && (
              <button
                className="mt-2 rounded-lg bg-red-600 px-3 py-1.5 text-xs text-white hover:bg-red-700"
                onClick={() => setShowSettings(true)}
              >
                前往设置更新引擎
              </button>
            )}
          </div>
        )}

        {/* 解析结果：单视频 */}
        {result?.kind === "video" && (
          <div className="flex flex-col gap-3 rounded-2xl border border-zinc-200 bg-white p-4 shadow-sm">
            <div className="flex gap-4">
              {result.info.thumbnail && (
                <img
                  src={result.info.thumbnail}
                  className="h-24 w-40 shrink-0 rounded-lg object-cover"
                  alt=""
                />
              )}
              <div className="min-w-0 flex-1">
                <div className="line-clamp-2 text-sm font-medium">{result.info.title}</div>
                <div className="mt-1 text-xs text-zinc-500">
                  {result.info.uploader && <span>{result.info.uploader} · </span>}
                  {formatDuration(result.info.duration)}
                </div>
              </div>
            </div>
            <FormatPicker info={result.info} value={format} onChange={setFormat} />
            <button
              className="self-end rounded-lg bg-blue-600 px-6 py-2 text-sm font-medium text-white hover:bg-blue-700"
              onClick={download}
            >
              下载
            </button>
          </div>
        )}

        {/* 解析结果：播放列表 */}
        {result?.kind === "playlist" && (
          <div className="rounded-2xl border border-zinc-200 bg-white p-4 shadow-sm">
            <div className="mb-2 flex items-center justify-between">
              <div className="text-sm font-medium">
                {result.info.title}
                <span className="ml-2 text-xs text-zinc-500">
                  已选 {checked.size} / {result.info.entries.length} 个视频
                </span>
              </div>
              <button
                className="text-xs text-blue-600 hover:underline"
                onClick={() =>
                  setChecked(
                    checked.size === result.info.entries.length
                      ? new Set()
                      : new Set(result.info.entries.map((_, i) => i)),
                  )
                }
              >
                {checked.size === result.info.entries.length ? "全不选" : "全选"}
              </button>
            </div>
            <div className="max-h-64 overflow-y-auto rounded-lg border border-zinc-100">
              {result.info.entries.map((e, i) => (
                <label
                  key={i}
                  className="flex cursor-pointer items-center gap-2 border-b border-zinc-100 px-3 py-2 text-sm last:border-b-0 hover:bg-zinc-50"
                >
                  <input type="checkbox" checked={checked.has(i)} onChange={() => toggleChecked(i)} />
                  <span className="min-w-0 flex-1 truncate" title={e.title}>
                    {e.title}
                  </span>
                  <span className="shrink-0 text-xs text-zinc-400">
                    {formatDuration(e.duration)}
                  </span>
                </label>
              ))}
            </div>
            <div className="mt-3 flex items-center gap-2">
              <FormatSelect value={format} onChange={setFormat} maxHeight={null} />
              <button
                className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700 disabled:opacity-50"
                disabled={checked.size === 0}
                onClick={download}
              >
                下载所选（{checked.size}）
              </button>
            </div>
          </div>
        )}

        {/* 任务列表 */}
        <section>
          <h2 className="mb-2 text-sm font-semibold text-zinc-600">下载任务</h2>
          <TaskList
            tasks={tasks}
            onRemoved={(id) => setTasks((prev) => prev.filter((t) => t.id !== id))}
          />
        </section>
      </main>

      {showAbout && <AboutModal engine={engine} onClose={() => setShowAbout(false)} />}

      {showSettings && settings && (
        <SettingsModal
          settings={settings}
          engine={engine}
          onClose={() => setShowSettings(false)}
          onSaved={(s) => {
            setSettings(s);
            setFormat(s.defaultFormat);
          }}
          onEngineUpdated={refreshEngine}
        />
      )}
    </div>
  );
}
