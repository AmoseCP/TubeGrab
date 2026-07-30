import { api } from "../api";
import {
  formatEta,
  formatLabel,
  formatSpeed,
  Task,
  TaskStatus,
} from "../types";

const STATUS_TEXT: Record<TaskStatus, string> = {
  queued: "排队中",
  downloading: "下载中",
  merging: "处理中",
  done: "完成",
  failed: "失败",
  canceled: "已取消",
};

const STATUS_COLOR: Record<TaskStatus, string> = {
  queued: "bg-zinc-100 text-zinc-600",
  downloading: "bg-blue-100 text-blue-700",
  merging: "bg-amber-100 text-amber-700",
  done: "bg-green-100 text-green-700",
  failed: "bg-red-100 text-red-700",
  canceled: "bg-zinc-100 text-zinc-500",
};

function TaskRow({ task }: { task: Task }) {
  const active = task.status === "downloading" || task.status === "merging";
  return (
    <div className="rounded-xl border border-zinc-200 bg-white p-3 shadow-sm">
      <div className="flex items-center gap-3">
        {task.thumbnail ? (
          <img
            src={task.thumbnail}
            className="h-12 w-20 shrink-0 rounded-md object-cover"
            alt=""
          />
        ) : (
          <div className="h-12 w-20 shrink-0 rounded-md bg-zinc-200" />
        )}
        <div className="min-w-0 flex-1">
          <div className="truncate text-sm font-medium text-zinc-800" title={task.title}>
            {task.title}
          </div>
          <div className="mt-0.5 flex items-center gap-2 text-xs text-zinc-500">
            <span className={`rounded-full px-2 py-0.5 ${STATUS_COLOR[task.status]}`}>
              {STATUS_TEXT[task.status]}
            </span>
            <span>{formatLabel(task.format)}</span>
            {task.status === "downloading" && (
              <>
                <span>{formatSpeed(task.speed)}</span>
                <span>{formatEta(task.eta)}</span>
              </>
            )}
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          {task.status === "done" && task.filepath && (
            <button
              className="rounded-lg border border-zinc-300 px-2.5 py-1 text-xs text-zinc-700 hover:bg-zinc-100"
              onClick={() => api.openInFolder(task.filepath!)}
            >
              打开所在文件夹
            </button>
          )}
          {(task.status === "failed" || task.status === "canceled") && (
            <button
              className="rounded-lg border border-zinc-300 px-2.5 py-1 text-xs text-zinc-700 hover:bg-zinc-100"
              onClick={() => api.retryTask(task.id)}
            >
              重试
            </button>
          )}
          {(task.status === "downloading" || task.status === "merging" || task.status === "queued") && (
            <button
              className="rounded-lg border border-zinc-300 px-2.5 py-1 text-xs text-zinc-700 hover:bg-zinc-100"
              onClick={() => api.cancelTask(task.id)}
            >
              取消
            </button>
          )}
          {!active && (
            <button
              className="rounded-lg px-2 py-1 text-xs text-zinc-400 hover:text-red-600"
              title="移除"
              onClick={() => api.removeTask(task.id)}
            >
              ✕
            </button>
          )}
        </div>
      </div>
      {active && (
        <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-zinc-100">
          <div
            className={`h-full rounded-full transition-[width] duration-200 ${
              task.status === "merging" ? "bg-amber-400" : "bg-blue-500"
            }`}
            style={{ width: `${task.status === "merging" ? 100 : task.percent}%` }}
          />
        </div>
      )}
      {task.status === "failed" && task.error && (
        <div className="mt-2 whitespace-pre-wrap rounded-lg bg-red-50 p-2 text-xs text-red-700">
          {task.error}
        </div>
      )}
    </div>
  );
}

export default function TaskList({ tasks }: { tasks: Task[] }) {
  if (tasks.length === 0) {
    return (
      <div className="rounded-xl border border-dashed border-zinc-300 p-8 text-center text-sm text-zinc-400">
        暂无下载任务，粘贴链接开始下载
      </div>
    );
  }
  // 新任务在前
  const sorted = [...tasks].sort((a, b) => b.id - a.id);
  return (
    <div className="flex flex-col gap-2">
      {sorted.map((t) => (
        <TaskRow key={t.id} task={t} />
      ))}
    </div>
  );
}
