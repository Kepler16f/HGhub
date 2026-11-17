/* 更新中心：前台窗口（发现新版本 / 下载进度 / 重启安装）+ 后台悬浮进度通知。
 *
 * 只负责渲染；状态与下载任务都在 src/updateStore.ts，所以关掉窗口、切页之后
 * 下载照常继续，右下角进度条仍然可见。 */
import { useEffect } from "react";
import { Modal } from "./ui";
import {
  autoCheckOnStart,
  checkForUpdate,
  formatBytes,
  installAndRestart,
  openReleasePage,
  openUpdateDialog,
  startDownload,
  useUpdateState,
  type UpdateState,
} from "../updateStore";
import { IClose, IDownload, IRefresh } from "../icons";

function percent(s: UpdateState): number | null {
  if (!s.total || s.total <= 0) return null;
  return Math.min(100, Math.max(0, (s.downloaded / s.total) * 100));
}

function Progress({ s }: { s: UpdateState }) {
  const pct = percent(s);
  return (
    <div className="mt-3">
      <div className="h-[6px] rounded-full bg-s2 overflow-hidden">
        {pct === null ? (
          <div className="h-full w-1/3 bg-primary/70 animate-pulse" />
        ) : (
          <div className="h-full bg-primary transition-[width] duration-150" style={{ width: `${pct.toFixed(1)}%` }} />
        )}
      </div>
      <div className="mt-1.5 flex items-center justify-between text-[10.5px] text-mut">
        <span>
          {pct === null
            ? `已下载 ${formatBytes(s.downloaded)}`
            : `${pct.toFixed(0)}% · ${formatBytes(s.downloaded)}${s.total ? ` / ${formatBytes(s.total)}` : ""}`}
        </span>
        <span>可切到后台继续下载</span>
      </div>
    </div>
  );
}

function DialogBody({ s }: { s: UpdateState }) {
  if (s.phase === "checking") {
    return (
      <div className="p-6 flex items-center gap-3">
        <div className="w-6 h-6 spinner" />
        <div className="text-[13px] text-on">正在检查更新…</div>
      </div>
    );
  }

  if (s.phase === "legacy") {
    return (
      <div className="p-5">
        <div className="text-[15px] font-bold text-on">发现新版本 v{s.version}</div>
        <p className="text-[11.5px] text-mut mt-1.5">
          当前 v{s.currentVersion}
          {s.date ? ` · ${s.date.slice(0, 10)}` : ""}
        </p>
        {!!s.notes && (
          <div className="mt-3 max-h-[180px] scroll-y rounded-(--r-sm) bg-s2 p-3 text-[11px] text-mut whitespace-pre-wrap leading-relaxed">{s.notes}</div>
        )}
        <div className="mt-3 rounded-(--r-sm) bg-s2 p-3 text-[10.5px] text-mut leading-relaxed">
          该版本暂未提供应用内更新包，请前往 GitHub Release 下载安装。
        </div>
        <div className="flex gap-2 mt-4">
          <button onClick={() => openUpdateDialog(false)} className="flex-1 h-9 rounded-(--r-ctl) bg-s3 text-[12.5px] text-on/80 pressable">稍后</button>
          <button onClick={() => void openReleasePage()} className="flex-1 h-9 rounded-(--r-ctl) bg-primary text-[12.5px] font-semibold text-on-primary pressable">前往 Release</button>
        </div>
      </div>
    );
  }

  if (s.phase === "error") {
    return (
      <div className="p-5">
        <div className="text-[15px] font-bold text-on">更新失败</div>
        <p className="text-[11.5px] text-mut mt-1.5 break-words">{s.error || "未知错误"}</p>
        <div className="flex gap-2 mt-4">
          <button onClick={() => openUpdateDialog(false)} className="flex-1 h-9 rounded-(--r-ctl) bg-s3 text-[12.5px] text-on/80 pressable">关闭</button>
          <button onClick={() => { if (s.version && !s.url) void startDownload(false); else void checkForUpdate(true); }} className="flex-1 h-9 rounded-(--r-ctl) bg-primary text-[12.5px] font-semibold text-on-primary pressable">重试</button>
        </div>
      </div>
    );
  }

  if (s.phase === "installing") {
    return (
      <div className="p-6 flex items-center gap-3">
        <div className="w-6 h-6 spinner" />
        <div>
          <div className="text-[13px] text-on">正在安装更新…</div>
          <div className="text-[10.5px] text-mut mt-0.5">应用即将自动重启</div>
        </div>
      </div>
    );
  }

  if (s.phase === "ready") {
    return (
      <div className="p-5">
        <div className="flex items-center gap-2 text-[15px] font-bold text-on">
          <ICheckMark />
          更新已下载完成
        </div>
        <p className="text-[11.5px] text-mut mt-1.5">
          v{s.currentVersion} → <b className="text-on">v{s.version}</b>
        </p>
        <div className="mt-3 rounded-(--r-sm) bg-s2 p-3 text-[11px] text-mut leading-relaxed">
          更新包已校验并保存在本地，重启后自动完成安装。
        </div>
        <div className="flex gap-2 mt-4">
          <button onClick={() => openUpdateDialog(false)} className="flex-1 h-9 rounded-(--r-ctl) bg-s3 text-[12.5px] text-on/80 pressable">稍后</button>
          <button onClick={() => void installAndRestart()} className="flex-1 h-9 rounded-(--r-ctl) bg-primary text-[12.5px] font-semibold text-on-primary pressable">立即重启安装</button>
        </div>
      </div>
    );
  }

  if (s.phase === "downloading") {
    return (
      <div className="p-5">
        <div className="flex items-center gap-2 text-[15px] font-bold text-on">
          <IDownload size={17} className="text-primary" />
          正在下载更新 v{s.version}
        </div>
        <Progress s={s} />
        <div className="flex gap-2 mt-4">
          <button onClick={() => openUpdateDialog(false)} className="flex-1 h-9 rounded-(--r-ctl) bg-s3 text-[12.5px] text-on/80 pressable">后台继续</button>
        </div>
        <div className="text-[10px] text-dim mt-2">下载在后台进行，完成后会提示重启安装。</div>
      </div>
    );
  }

  // available
  return (
    <div className="p-5">
      <div className="text-[15px] font-bold text-on">发现新版本 v{s.version}</div>
      <p className="text-[11.5px] text-mut mt-1.5">
        当前 v{s.currentVersion}
        {s.date ? ` · ${s.date.slice(0, 10)}` : ""}
      </p>
      {!!s.notes && (
        <div className="mt-3 max-h-[160px] scroll-y rounded-(--r-sm) bg-s2 p-3 text-[11px] text-mut whitespace-pre-wrap leading-relaxed">{s.notes}</div>
      )}
      <div className="flex gap-2 mt-4">
        <button onClick={() => openUpdateDialog(false)} className="flex-1 h-9 rounded-(--r-ctl) bg-s3 text-[12.5px] text-on/80 pressable">稍后</button>
        <button onClick={() => void startDownload(true)} className="flex-1 h-9 rounded-(--r-ctl) bg-s3 text-[12.5px] text-on/80 pressable">后台下载</button>
        <button onClick={() => void startDownload(false)} className="flex-1 h-9 rounded-(--r-ctl) bg-primary text-[12.5px] font-semibold text-on-primary pressable">立即下载</button>
      </div>
    </div>
  );
}

function ICheckMark() {
  return (
    <span className="w-5 h-5 rounded-full bg-primary/15 text-primary flex items-center justify-center">
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.6" strokeLinecap="round" strokeLinejoin="round">
        <path d="M5 12.5l4.5 4.5L19 7" />
      </svg>
    </span>
  );
}

/* 后台下载通知：窗口关掉后仍显示进度，点击可重新打开更新窗口 */
function Banner({ s }: { s: UpdateState }) {
  if (s.dialogOpen) return null;
  const active = s.phase === "downloading" || s.phase === "ready" || s.phase === "installing";
  if (!active) return null;
  const pct = percent(s);
  return (
    <button
      onClick={() => openUpdateDialog(true)}
      className="fixed z-[60] right-3 bottom-[calc(var(--sab,0px)+64px)] md:bottom-4 md:right-4 w-[280px] text-left bg-s3/95 backdrop-blur-md rounded-(--r-md) elev-3 p-3 pressable a-rise"
      aria-live="polite"
    >
      <div className="flex items-center gap-2.5">
        <span className="w-8 h-8 rounded-full bg-primary/15 text-primary flex items-center justify-center shrink-0">
          <IDownload size={15} />
        </span>
        <div className="min-w-0 flex-1">
          <div className="text-[12px] text-on font-semibold truncate">
            {s.phase === "ready" ? `更新 v${s.version} 已就绪` : s.phase === "installing" ? "正在安装更新…" : `正在下载 v${s.version}`}
          </div>
          <div className="text-[10px] text-mut mt-0.5">
            {s.phase === "ready" ? "点击重启并安装" : pct === null ? formatBytes(s.downloaded) : `${pct.toFixed(0)}% · ${formatBytes(s.downloaded)}`}
          </div>
        </div>
        {s.phase === "downloading" && <IRefresh size={14} className="text-mut shrink-0 animate-spin" />}
        {s.phase === "ready" && <span className="text-[10px] text-primary shrink-0">查看</span>}
      </div>
      {s.phase === "downloading" && (
        <div className="mt-2 h-[4px] rounded-full bg-s2 overflow-hidden">
          {pct === null ? (
            <div className="h-full w-1/3 bg-primary/70 animate-pulse" />
          ) : (
            <div className="h-full bg-primary transition-[width] duration-150" style={{ width: `${pct.toFixed(1)}%` }} />
          )}
        </div>
      )}
    </button>
  );
}

export default function UpdateCenter() {
  const s = useUpdateState();

  useEffect(() => {
    void autoCheckOnStart();
    // 只跑一次：启动后自动检查；检查失败静默
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const showDialog = s.dialogOpen && s.phase !== "idle";
  const close = () => {
    if (s.phase === "installing") return;
    openUpdateDialog(false);
  };

  return (
    <>
      <Modal open={showDialog} onClose={close} width={392}>
        {showDialog && (
          <div className="relative">
            {(s.phase === "available" || s.phase === "legacy") && (
              <button onClick={close} aria-label="关闭" className="absolute right-3 top-3 w-8 h-8 rounded-full flex items-center justify-center text-mut hover:bg-s2 pressable z-10">
                <IClose size={15} />
              </button>
            )}
            <DialogBody s={s} />
          </div>
        )}
      </Modal>
      <Banner s={s} />
    </>
  );
}
