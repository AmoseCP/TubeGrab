import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { openUrl } from "@tauri-apps/plugin-opener";
import { EngineInfo } from "../types";

interface Props {
  engine: EngineInfo | null;
  onClose: () => void;
}

export default function AboutModal({ engine, onClose }: Props) {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40" onClick={onClose}>
      <div
        className="w-[480px] max-w-[92vw] rounded-2xl bg-white p-5 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-base font-semibold text-zinc-800">关于 TubeGrab</h2>
          <button className="text-zinc-400 hover:text-zinc-600" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="flex flex-col gap-3 text-sm">
          <div className="rounded-xl bg-zinc-50 p-3">
            <div className="flex justify-between py-0.5">
              <span className="text-zinc-500">软件版本</span>
              <span className="font-mono">{version || "…"}</span>
            </div>
            <div className="flex justify-between py-0.5">
              <span className="text-zinc-500">下载引擎 yt-dlp</span>
              <span className="font-mono">{engine?.ytdlp_version ?? "未知"}</span>
            </div>
            <div className="flex justify-between py-0.5">
              <span className="text-zinc-500">ffmpeg</span>
              <span className="font-mono">{engine?.ffmpeg_version ?? "未知"}</span>
            </div>
            <div className="flex justify-between py-0.5">
              <span className="text-zinc-500">开发者</span>
              <button
                className="font-mono text-blue-600 hover:underline"
                onClick={() => openUrl("https://t.me/Dingjin2025")}
                title="在 Telegram 上联系开发者"
              >
                Telegram: @Dingjin2025
              </button>
            </div>
          </div>

          <div className="rounded-xl border border-amber-200 bg-amber-50 p-3 text-xs leading-relaxed text-amber-900">
            <div className="mb-1 font-semibold">免责声明</div>
            <p>
              本软件仅供个人学习、研究及下载自己有权访问的内容使用。请遵守 YouTube
              服务条款及您所在地区的版权法律法规，请勿下载、传播受版权保护的内容，
              勿将本软件用于任何商业或侵权用途。
            </p>
            <p className="mt-1">
              使用本软件产生的一切后果由使用者自行承担，开发者不承担任何法律责任。
              本软件不提供绕过登录、会员或年龄限制的功能。
            </p>
          </div>
        </div>

        <div className="mt-4 flex justify-end">
          <button
            className="rounded-lg border border-zinc-300 px-4 py-2 text-sm hover:bg-zinc-50"
            onClick={onClose}
          >
            关闭
          </button>
        </div>
      </div>
    </div>
  );
}
