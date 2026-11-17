/* 应用内更新状态：检查 → 后台下载（进度）→ 重启安装。
 *
 * 原生下载/安装交给 tauri-plugin-updater（Rust 侧），这里只维护一份
 * 可跨页面订阅的状态，让设置页、更新窗口和后台进度条共用同一个下载任务。
 * 移动端（Android / iOS）没有 updater 后端，回退到 GitHub Release 网页。 */
import { useSyncExternalStore } from "react";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { api, platform } from "./api";

export type UpdatePhase =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
  | "legacy" // 旧版 GitHub API 检查到的更新；没有可签名的应用内更新包
  | "error";

export type UpdateState = {
  phase: UpdatePhase;
  /** updater 清单里的版本 */
  version: string;
  currentVersion: string;
  notes: string;
  date: string;
  /** legacy 模式下的 Release 页面地址 */
  url: string;
  downloaded: number;
  total: number;
  error: string;
  dialogOpen: boolean;
  autoDownload: boolean;
  supported: boolean;
};

const AUTO_KEY = "hghub_auto_update";
const supported = platform === "windows" || platform === "macos" || platform === "linux";

function readAuto(): boolean {
  if (!supported) return false;
  try {
    return localStorage.getItem(AUTO_KEY) !== "0";
  } catch {
    return true;
  }
}

let autoDownload = readAuto();

let state: UpdateState = {
  phase: "idle",
  version: "",
  currentVersion: "",
  notes: "",
  date: "",
  url: "",
  downloaded: 0,
  total: 0,
  error: "",
  dialogOpen: false,
  autoDownload,
  supported,
};

const listeners = new Set<() => void>();

function emit(patch: Partial<UpdateState>) {
  state = { ...state, ...patch };
  listeners.forEach((fn) => fn());
}

export function subscribe(fn: () => void) {
  listeners.add(fn);
  return () => listeners.delete(fn);
}

export function getUpdateState(): UpdateState {
  return state;
}

export function useUpdateState(): UpdateState {
  return useSyncExternalStore(subscribe, getUpdateState, getUpdateState);
}

/** 当前持有的 Rust Updater 资源；只允许一个下载任务 */
let pending: Update | null = null;
let busy = false;
let lastProgressEmit = 0;

function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error && e.message) return e.message;
  return "更新失败，请稍后重试";
}

export function formatBytes(n: number): string {
  if (!n || n < 0) return "0 B";
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`;
  return `${(n / (1024 * 1024)).toFixed(1)} MB`;
}

export function setAutoDownload(on: boolean) {
  autoDownload = on;
  try {
    localStorage.setItem(AUTO_KEY, on ? "1" : "0");
  } catch {
    /* ignore */
  }
  emit({ autoDownload: on });
  if (on && state.phase === "idle" && supported) void checkForUpdate(false);
}

export function openUpdateDialog(open = true) {
  emit({ dialogOpen: open });
}

/**
 * 检查更新。manual=true 时会打开更新窗口；无更新时返回 "none" 由调用方提示。
 * 原生清单不可用（例如尚未发布 latest.json）时回退 GitHub Release 检查。
 */
export async function checkForUpdate(manual = true): Promise<"available" | "none" | "error"> {
  if (!supported) {
    if (manual) emit({ phase: "error", error: "当前平台请前往 Release 页面手动下载更新", dialogOpen: true });
    return "error";
  }
  if (busy) return "none";
  busy = true;
  emit({ phase: "checking", error: "", dialogOpen: manual });
  try {
    const up = await check({ timeout: 15_000 });
    if (up) {
      if (pending && pending.rid !== up.rid) void pending.close().catch(() => {});
      pending = up;
      emit({
        phase: "available",
        version: up.version,
        currentVersion: up.currentVersion || state.currentVersion,
        notes: up.body || "",
        date: up.date || "",
        url: "",
        downloaded: 0,
        total: 0,
        dialogOpen: manual,
      });
      // 自动模式：不弹窗，直接后台下载，完成后打开窗口提示重启
      if (!manual) void startDownload(true, true);
      return "available";
    }
    emit({ phase: "idle", dialogOpen: false, error: "" });
    return "none";
  } catch (e) {
    if (!manual) {
      // 自动检查失败（无清单 / 网络问题）保持安静，避免每次启动弹错误
      emit({ phase: "idle", dialogOpen: false });
      return "error";
    }
    try {
      const info = await api.checkUpdate();
      if (info.has_update) {
        emit({
          phase: "legacy",
          version: info.latest,
          currentVersion: info.current,
          notes: info.notes || "",
          date: info.published_at || "",
          url: info.url,
          dialogOpen: true,
        });
        return "available";
      }
      emit({ phase: "idle", dialogOpen: false, error: "" });
      return "none";
    } catch (e2) {
      emit({ phase: "error", error: errText(e2 || e), dialogOpen: true });
      return "error";
    }
  } finally {
    busy = false;
  }
}

/** 下载更新包；background=true 时关闭窗口，由后台进度条显示进度。 */
export async function startDownload(background = false, auto = false) {
  if (!pending || state.phase === "downloading" || state.phase === "installing") return;
  if (state.phase !== "available" && state.phase !== "error" && state.phase !== "legacy") return;
  lastProgressEmit = 0;
  emit({ phase: "downloading", dialogOpen: !background, downloaded: 0, total: 0, error: "" });
  try {
    await pending.download((ev: DownloadEvent) => {
      if (ev.event === "Started") {
        emit({ total: ev.data.contentLength || 0 });
      } else if (ev.event === "Progress") {
        const downloaded = state.downloaded + ev.data.chunkLength;
        const now = performance.now();
        // 分片事件很密，限制刷新频率，避免整页每 64KB 重渲染一次
        if (now - lastProgressEmit > 120) {
          lastProgressEmit = now;
          emit({ downloaded });
        } else {
          state = { ...state, downloaded };
        }
      }
    });
    emit({ phase: "ready", downloaded: state.total || state.downloaded, dialogOpen: auto });
  } catch (e) {
    emit({ phase: "error", error: errText(e), dialogOpen: true });
  }
}

/** 安装已下载的更新包并重启应用（macOS / Linux 需要重启才切换到新版本）。 */
export async function installAndRestart() {
  if (!pending || state.phase === "installing") return;
  if (state.phase !== "ready") return;
  emit({ phase: "installing", error: "" });
  try {
    await pending.install({ restartAfterInstall: false });
    await relaunch();
  } catch (e) {
    emit({ phase: "error", error: errText(e), dialogOpen: true });
  }
}

/** legacy：打开 GitHub Release 页面下载（没有可签名更新包时的兜底）。 */
export async function openReleasePage() {
  if (!state.url) return;
  try {
    await api.openUrl(state.url);
    emit({ dialogOpen: false });
  } catch (e) {
    emit({ error: errText(e) });
  }
}

/** 启动时自动检查（受设置开关控制），失败静默。 */
export async function autoCheckOnStart() {
  if (!supported || !autoDownload) return;
  if (state.phase !== "idle") return;
  await checkForUpdate(false);
}
