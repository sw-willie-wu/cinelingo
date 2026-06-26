import { reactive, readonly } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { open } from '@tauri-apps/plugin-dialog'
import {
  startMpv, loadFile, stopMpv,
  togglePause as mpvTogglePause,
  seekAbsolute, seekRelative,
  setVolume as mpvSetVolume, setMute,
  loadViaYtdl, setPause, onMpvEvent,
  type MpvState,
} from '../mpv'
import { resolveRemote, type ResolvedRemote } from './backend'
import { canonicalRemoteId } from './remoteUrl'
import { clampVolume } from './volume'
import { pickForPref } from './quality'
import { useSettings, whenSettingsReady } from './useSettings'

export type Source =
  | { kind: 'local'; id: string }
  | { kind: 'remote'; id: string; watchUrl: string; resolved: ResolvedRemote }
// resolving/loadError 供「解析中」與失敗的可見指示（spec §3/§4.1/§9.5）。
// qualitySwitch：YouTube 切畫質 reload 期間，凍住進度條/時間在原位（避免 reload 把 time-pos 歸 0 造成跳動）+ 顯示 spinner。
const source = reactive<{
  current: Source | null; resolving: boolean; loadError: string
  qualitySwitch: { active: boolean; posSec: number; posFrac: number; durSec: number }
}>({
  current: null, resolving: false, loadError: '',
  qualitySwitch: { active: false, posSec: 0, posFrac: 0, durSec: 0 },
})

let qsTimer: ReturnType<typeof setTimeout> | null = null
// 開始切畫質：凍結指示用的位置快照（reload 前 time-pos/duration 仍有效時擷取）。12s 安全清除。
function beginQualitySwitch(posSec: number, posFrac: number, durSec: number): void {
  source.qualitySwitch = { active: true, posSec, posFrac, durSec }
  if (qsTimer) clearTimeout(qsTimer)
  qsTimer = setTimeout(() => endQualitySwitch(), 12000)
}
// 結束切畫質（seek 落地後由 useQueue 延遲呼叫，或安全逾時）：放開凍結、隱藏 spinner。
function endQualitySwitch(): void {
  source.qualitySwitch.active = false
  if (qsTimer) { clearTimeout(qsTimer); qsTimer = null }
}

// 載入提示/錯誤：設訊息後 ~5s 自動淡掉（不長擋畫面中央）。
let errTimer: ReturnType<typeof setTimeout> | null = null
function clearErr(): void { source.loadError = ''; if (errTimer) { clearTimeout(errTimer); errTimer = null } }
function notify(msg: string): void {
  source.loadError = msg
  if (errTimer) clearTimeout(errTimer)
  errTimer = setTimeout(() => { source.loadError = ''; errTimer = null }, 3000)
}

interface PlayerState extends MpvState {
  fullscreen: boolean   // 非 mpv 屬性:由 Tauri 視窗追蹤
}

const state = reactive<PlayerState>({
  pause: null, path: null, timePos: null,
  duration: null, volume: null, mute: null,
  pausedForCache: null, cacheBufferingState: null, demuxerCacheTime: null,
  fullscreen: false,
})

let started = false

// seek 通知監聽（useSubtitles 註冊；解耦避免循環 import）。
const seekListeners: ((sec: number) => void)[] = []

// mpv 事件監聽（useQueue 註冊；解耦避免循環 import）。事件無檔案 identity → 由 useQueue 以 gen/entryId 關聯。
const endFileListeners: ((reason: string, entryId: number) => void)[] = []
const fileLoadedListeners: (() => void)[] = []
const startFileListeners: ((entryId: number) => void)[] = []
function onEndFile(cb: (reason: string, entryId: number) => void): void { endFileListeners.push(cb) }
function onFileLoaded(cb: () => void): void { fileLoadedListeners.push(cb) }
function onStartFile(cb: (entryId: number) => void): void { startFileListeners.push(cb) }
function setResolving(on: boolean): void { source.resolving = on }

function logErr(label: string) {
  return (e: unknown) => console.error(`[player] ${label} failed`, e)
}

const onState = (s: MpvState) => {
  state.pause = s.pause; state.path = s.path; state.timePos = s.timePos
  state.duration = s.duration; state.volume = s.volume; state.mute = s.mute
  state.pausedForCache = s.pausedForCache
  state.cacheBufferingState = s.cacheBufferingState
  state.demuxerCacheTime = s.demuxerCacheTime
}
function wireMpvEvents(): Promise<() => void> {
  return onMpvEvent((e) => {
    if (e.event === 'end-file') endFileListeners.forEach((f) => f(e.reason, e.playlist_entry_id))
    else if (e.event === 'file-loaded') fileLoadedListeners.forEach((f) => f())
    else if (e.event === 'start-file') startFileListeners.forEach((f) => f(e.playlist_entry_id))
  })
}

async function start(): Promise<void> {
  if (started) return            // init-once 守衛,避免重複註冊 observer
  started = true
  await whenSettingsReady()       // 先等設定載入，才讀得到 playback.videoOutput（重啟才生效）
  await startMpv(onState, useSettings().state.playback.videoOutput)
  await wireMpvEvents()
  // §3.10：mpv init 後套用全域影像/音訊調整一次（屬性跨 loadfile 持續）。
  // 動態 import 避免與 composable 互相靜態依賴造成循環。
  const { useVideoAdjust } = await import('./useVideoAdjust')
  const { useAudioAdjust } = await import('./useAudioAdjust')
  await useVideoAdjust().applyFromSettings().catch(logErr('applyVideoAdjust'))
  await useAudioAdjust().applyFromSettings().catch(logErr('applyAudioAdjust'))
}

async function togglePause(): Promise<void> {
  await mpvTogglePause().catch(logErr('togglePause'))
}

async function seekTo(sec: number): Promise<void> {
  if (state.duration == null) return        // 未載入 → no-op
  await seekAbsolute(sec).catch(logErr('seekTo'))
  seekListeners.forEach((f) => f(sec))
}

// 不受 duration 守衛擋的絕對 seek：畫質 reload 後在 file-loaded 立即還原位置用
// （此時 duration observer 常尚未回填 → seekTo 會誤判未載入而 no-op，導致從 0 播）。
async function seekToImmediate(sec: number): Promise<void> {
  await seekAbsolute(sec).catch(logErr('seekToImmediate'))
  seekListeners.forEach((f) => f(sec))
}

async function seekBy(delta: number): Promise<void> {
  if (state.path == null) return            // 未載入 → no-op
  await seekRelative(delta).catch(logErr('seekBy'))
  seekListeners.forEach((f) => f((state.timePos ?? 0) + delta))  // 目標位置(observer 尚未更新 timePos)
}

function onSeek(cb: (sec: number) => void): void { seekListeners.push(cb) }

async function setVolume(v: number): Promise<void> {
  await mpvSetVolume(clampVolume(v)).catch(logErr('setVolume'))
}

async function adjustVolume(delta: number): Promise<void> {
  await setVolume((state.volume ?? 0) + delta)
}

async function toggleMute(): Promise<void> {
  await setMute(!(state.mute ?? false)).catch(logErr('toggleMute'))
}

async function setFullscreen(on: boolean): Promise<void> {
  try {
    await getCurrentWebviewWindow().setFullscreen(on)
    state.fullscreen = on
  } catch (e) {
    logErr('setFullscreen')(e)              // 失敗則不更新旗標(維持一致)
  }
}

async function toggleFullscreen(): Promise<void> { await setFullscreen(!state.fullscreen) }
async function exitFullscreen(): Promise<void> { if (state.fullscreen) await setFullscreen(false) }

// 多選開檔：僅回傳選取路徑，交呼叫端（ControlBar）經後端展開 + useQueue.enqueueItems（避免 usePlayer 反向依賴 useQueue）。
async function openFile(): Promise<string[]> {
  const sel = await open({ multiple: true, directory: false })
  if (!sel) return []
  return Array.isArray(sel) ? sel : [sel]
}

async function loadPath(p: string): Promise<boolean> {
  source.current = { kind: 'local', id: p }
  clearErr()
  try { await loadFile(p); return true } catch (e) { logErr('loadPath')(e); return false } // 同步/命令錯；缺檔的非同步失敗走 end-file{error}
}

// 播放「已解析」的 remote 來源：同步先設 source.current（保字幕不變式）再載入。供 useQueue gen-gate 後呼叫。
async function playResolvedRemote(canonicalId: string, r: ResolvedRemote): Promise<boolean> {
  if (r.isLive) { notify('本版不支援直播'); return false }
  source.current = { kind: 'remote', id: canonicalId, watchUrl: canonicalId, resolved: r } // canonical 鍵：同片不同 URL 形式共用快取/記憶
  clearErr()
  // 播放交給 mpv 內建 ytdl 重解 URL（不限速、可 cold seek）；用 pickVideoFormat 選出的 itag pin 畫質。
  // canonicalId 即 watch URL（mpv ytdl 直接吃）。無 DASH 影軌（純 muxed / 直連媒體）→ 退回 mpv 自選 best。
  const pref = useSettings().state.youtube.quality
  const v = pickForPref(r.videos, pref)
  if (v || r.playbackUrl || r.audioUrl) {
    const fmt = v ? `${v.itag}+bestaudio/bestvideo+bestaudio/best` : 'bestvideo+bestaudio/best'
    await loadViaYtdl(canonicalId, fmt).catch(logErr('loadViaYtdl'))
    return true
  }
  notify('找不到可播放的影音來源'); return false
}

async function loadUrl(watchUrl: string): Promise<{ ok: boolean; reason?: string }> {
  setResolving(true); clearErr()
  let r: ResolvedRemote
  try { r = await resolveRemote(watchUrl) }
  catch { setResolving(false); notify('無法解析此連結（不支援的網站或無法存取）'); return { ok: false, reason: 'resolve failed' } }
  setResolving(false)
  const ok = await playResolvedRemote(canonicalRemoteId(watchUrl), r)
  return ok ? { ok: true } : { ok: false, reason: 'live/no-url' }
}

async function shutdown(): Promise<void> { await stopMpv() }

/** 同片換畫質：用新 itag 重載（不動 source.current → canonicalId 恆定，識別冪等成立）。 */
async function reloadQualityFmt(quality: 'auto' | number): Promise<boolean> {
  const cur = source.current
  if (!cur || cur.kind !== 'remote') return false
  const v = pickForPref(cur.resolved.videos, quality)
  const fmt = v ? `${v.itag}+bestaudio/bestvideo+bestaudio/best` : 'bestvideo+bestaudio/best'
  await loadViaYtdl(cur.watchUrl, fmt).catch(logErr('reloadQualityFmt'))
  return true
}
async function setPauseTrue(): Promise<void> { await setPause(true).catch(logErr('setPauseTrue')) }

export function usePlayer() {
  return {
    state: readonly(state),
    source: readonly(source),
    start, shutdown,
    togglePause, seekTo, seekToImmediate, seekBy, onSeek,
    setVolume, adjustVolume, toggleMute,
    toggleFullscreen, exitFullscreen,
    openFile, loadPath, loadUrl, playResolvedRemote, notify,
    setResolving, onEndFile, onFileLoaded, onStartFile,
    reloadQualityFmt, setPauseTrue,
    beginQualitySwitch, endQualitySwitch,
  }
}
