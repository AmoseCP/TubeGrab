import { formatBitrate, formatSize, VideoInfo } from "../types";

interface Props {
  info: VideoInfo;
  value: string; // "mp4-<height>" | "mp3" | "m4a"
  onChange: (v: string) => void;
}

/** 解析卡片上的格式选择面板：视频/音频切换，列出实际可用的分辨率与码率。 */
export default function FormatPicker({ info, value, onChange }: Props) {
  const isVideo = value.startsWith("mp4-");
  const hasVideo = info.videoOptions.length > 0;

  function switchToVideo() {
    if (!isVideo && hasVideo) onChange(`mp4-${info.videoOptions[0].height}`);
  }
  function switchToAudio() {
    if (isVideo) onChange("m4a");
  }

  const abr = formatBitrate(info.audio.abr);
  const audioOptions = [
    {
      value: "m4a",
      name: "M4A",
      desc: `原始流，更快无损${abr ? ` · ${abr}` : ""}`,
      size: formatSize(info.audio.filesize),
    },
    {
      value: "mp3",
      name: "MP3",
      desc: `通用格式，转码${abr ? ` · 源音质 ${abr}` : ""}`,
      size: formatSize(info.audio.filesize),
    },
  ];

  const tabClass = (active: boolean) =>
    `flex-1 rounded-lg px-3 py-1.5 text-sm font-medium transition-colors ${
      active ? "bg-white text-zinc-900 shadow-sm" : "text-zinc-500 hover:text-zinc-700"
    }`;

  const rowClass = (active: boolean) =>
    `flex w-full items-center justify-between rounded-lg border px-3 py-2 text-left text-sm transition-colors ${
      active
        ? "border-blue-500 bg-blue-50 text-blue-800"
        : "border-zinc-200 bg-white text-zinc-700 hover:border-zinc-300 hover:bg-zinc-50"
    }`;

  return (
    <div className="flex flex-col gap-2">
      <div className="flex rounded-xl bg-zinc-100 p-1">
        <button className={tabClass(isVideo)} onClick={switchToVideo} disabled={!hasVideo}>
          视频 MP4{!hasVideo ? "（无视频流）" : ""}
        </button>
        <button className={tabClass(!isVideo)} onClick={switchToAudio}>
          音频
        </button>
      </div>

      {isVideo ? (
        <div className="grid grid-cols-2 gap-1.5 sm:grid-cols-3">
          {info.videoOptions.map((o) => {
            const v = `mp4-${o.height}`;
            return (
              <button key={o.height} className={rowClass(value === v)} onClick={() => onChange(v)}>
                <span className="font-medium">{o.height}p</span>
                <span className="ml-2 text-right text-xs opacity-70">
                  {formatBitrate(o.tbr)}
                  {o.filesize ? <br /> : null}
                  {formatSize(o.filesize)}
                </span>
              </button>
            );
          })}
        </div>
      ) : (
        <div className="flex flex-col gap-1.5">
          {audioOptions.map((o) => (
            <button key={o.value} className={rowClass(value === o.value)} onClick={() => onChange(o.value)}>
              <span>
                <span className="font-medium">{o.name}</span>
                <span className="ml-2 text-xs opacity-70">{o.desc}</span>
              </span>
              <span className="text-xs opacity-70">{o.size}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
