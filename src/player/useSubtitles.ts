import { ref, reactive, readonly, computed, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { upsertCue, upsertCues, selectCueAt, parseSubtitle, FINALS_CAP, type Cue } from './subtitles'
import { currentAudioFfIndex } from '../mpv'
import { usePlayer } from './usePlayer'
import { useSettings } from './useSettings'
import { langToWhisper } from './langs'
import { readTextFile, listSidecarSubs, loadSubMemory, saveSubMemory, startExternalTranscription, stopExternalTranscription, type SidecarSub } from './backend'
import { useAudioSource } from './useAudioSource'
import { normKey, trackToStored, restoreTrackSource, coerceStoredEntry, type StoredEntry } from './subMemory'
import { clampSecondaryToPrimary as clampSec, pickCcRestore, type CcSnapshot } from './ccRestore'

interface Progress { phase: string; done: number; total: number | null; message: string }
export type TrackName = 'primary' | 'secondary'
interface TrackSel { source: string; delaySec: number } // source: 'off' | 'live' | fileId

// live 來源
const liveCues = ref<Cue[]>([])
const liveInterim = ref<Cue | null>(null)
const noClock = ref(false)
const sessionId = ref('')
const frontierSec = ref(0)
const lang = ref<string | null>(null)
const promptText = ref<string | null>(null)
const model = ref('turbo')
const progress = ref<Progress | null>(null)
// 已載入外部檔
const files = ref<{ id: string; name: string; cues: Cue[]; path: string | null; manual: boolean }[]>([])
let fileCounter = 0
// startForCurrent 世代權杖：await resolveFfIndex 期間若被 stop/再啟動取代，過期 start 直接放棄（防殭屍轉寫/換檔搶跑）。
let startGen = 0
// onFileChanged 世代權杖：Part A/B 讓它變成多次 await 的長流程；每個 await 後若被新檔取代即 bail。
// 與 startGen 是不同關注點（一守 files 填充、一守轉寫啟動），各自獨立。
let fileGen = 0
// Part B：per-video 記憶（normKey(path) → entry）。在 ensureWired 載入一次。
// 以 Record<string, unknown> 儲存，讀取時一律經 coerceStoredEntry 塑型，防磁碟格式不完整觸發 TypeError。
let memory: Record<string, unknown> = {}
let restoreDepth = 0 // 還原期間不要把還原結果又寫回（counter 而非 bool，支援重疊還原正確計數）
let saveTimer: ReturnType<typeof setTimeout> | null = null
let lastFileId = ''
let lastSubSnapshot: CcSnapshot | null = null

function saveMemory(): Promise<void> {
  return saveSubMemory(memory).catch((e) => { console.warn('[subMemory] save failed', e) })
}
function scheduleSave(): void {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(() => { saveTimer = null; void saveMemory() }, 300)
}
/** 關閉前/需要即時落地時呼叫：取消 debounce 並同步寫一次。 */
function flushMemory(): Promise<void> {
  if (saveTimer) { clearTimeout(saveTimer); saveTimer = null }
  return saveMemory()
}

/** 用目前狀態寫入 memory[normKey(currentPath)]；兩軌 off 且無 manualFiles → 刪 key。debounce 存檔。 */
function recordCurrent(): void {
  if (restoreDepth > 0) return
  const path = usePlayer().source.current?.id ?? '' // canonical id：local=檔路徑、remote=watchUrl（非會過期的 state.path）
  if (!path) return
  const key = normKey(path)
  const manualFiles = files.value.filter((f) => f.manual && f.path).map((f) => f.path as string)
  const primary = trackToStored(tracks.primary, files.value)
  const secondary = trackToStored(tracks.secondary, files.value)
  const empty = manualFiles.length === 0 && primary.source === 'off' && secondary.source === 'off'
  if (empty) delete memory[key]
  else memory[key] = { manualFiles, primary, secondary }
  scheduleSave()
}
// 雙軌
const tracks = reactive<{ primary: TrackSel; secondary: TrackSel }>({
  primary: { source: 'off', delaySec: 0 },
  secondary: { source: 'off', delaySec: 0 },
})

const liveNeeded = computed(() => tracks.primary.source === 'live' || tracks.secondary.source === 'live')

let wirePromise: Promise<void> | null = null
function ensureWired(): Promise<void> {
  if (!wirePromise) {
    wirePromise = (async () => {
      try { memory = (await loadSubMemory()) as Record<string, unknown> } catch { memory = {} }
      await listen<Cue>('sub-cue', (e) => {
        if (e.payload.sessionId !== sessionId.value) return
        if (!liveNeeded.value) return  // CC 已關：拒絕後到的 cue，避免 race 黏畫面
        if (e.payload.status === 'interim') {
          liveInterim.value = e.payload.sourceText.trim() ? e.payload : null
        } else {
          liveCues.value = upsertCue(liveCues.value, e.payload)
          // FINALS_CAP 只在 no-clock 套用（clock 模式長片 seek-back 不可丟舊 cue；spec §4.3）
          if (noClock.value && liveCues.value.length > FINALS_CAP) {
            liveCues.value = liveCues.value.slice(liveCues.value.length - FINALS_CAP)
          }
        }
      })
      await listen<Cue[]>('sub-cue-batch', (e) => {
        if (!liveNeeded.value) return
        const seed = e.payload.filter((c) => c.sessionId === sessionId.value)
        if (seed.length) liveCues.value = upsertCues(liveCues.value, seed)
      })
      await listen<{ sessionId: string; noClock: boolean }>('sub-session-reset', (e) => {
        sessionId.value = e.payload.sessionId
        noClock.value = e.payload.noClock
        liveCues.value = []
        liveInterim.value = null
        frontierSec.value = 0
      })
      await listen<Progress>('sub-progress', (e) => {
        progress.value = e.payload
        if (e.payload.phase === 'transcribe') frontierSec.value = e.payload.done
      })
      usePlayer().onSeek((sec: number) => { if (liveNeeded.value) invoke('notify_seek', { sec }) })
      const s = useSettings().state
      // 啟用中改 model/語言 → 重啟
      watch(() => [s.liveSubs.model, s.liveSubs.sourceLang, s.liveSubs.translateEnabled, s.liveSubs.translateTo] as const, () => {
        if (liveNeeded.value) startForCurrent().catch(() => {})
      })
      // 任一軌進/離 live → 起/停轉寫（startForCurrent 依 armed 狀態分派外部/影片路徑）
      watch(liveNeeded, (need) => {
        if (need) startForCurrent().catch(() => {})
        else {
          startGen++
          // armed 模式：stop_external_transcription 僅 set_transcribe(false)，保留 capture loop；
          // 非 armed（影片）：stop_transcription = shutdown()，殺整個 session。
          if (useAudioSource().armed.value) {
            stopExternalTranscription().catch(() => {})
            // noClock 不重置：arm 時 sub-session-reset 已設 noClock=true；start_external_transcription
            // 不重發 reset，若此處清掉 false，重開 CC 後字幕走時間模式永遠顯示不出。
          } else {
            invoke('stop_transcription').catch(() => {})
            noClock.value = false
          }
          liveCues.value = []
          liveInterim.value = null
        }
      })
      const audioSrc = useAudioSource()
      // armed 解除 → live 軌關閉（觸發 stop）；armed 啟用 + liveNeeded 已開 → 主動重啟（避免先開 CC 再 arm 時無字幕）
      watch(audioSrc.armed, (armed) => {
        if (!armed) {
          if (tracks.primary.source === 'live') tracks.primary.source = 'off'
          if (tracks.secondary.source === 'live') tracks.secondary.source = 'off'
        } else if (liveNeeded.value) {
          startForCurrent().catch(() => {})
        }
      })
      // 外部音源切換（已在轉寫中）→ 以新音源重啟
      watch(audioSrc.current, (newSrc, oldSrc) => {
        if (newSrc !== null && oldSrc !== null && liveNeeded.value) {
          startForCurrent().catch(() => {})
        }
      })
    })()
  }
  return wirePromise
}

// track-list 在 file-loaded 才齊 → 重試解析當前音軌 ff-index。
async function resolveFfIndex(): Promise<number | null> {
  for (let i = 0; i < 30; i++) {
    const ff = await currentAudioFfIndex()
    if (ff != null) return ff
    await new Promise((r) => setTimeout(r, 100))
  }
  return null
}

// 呼叫當下自種 model + lang + prompt（取代舊 enable() 的 seeding）。
// armed + 無影片 → 外部轉寫；否則走影片路徑。
async function startForCurrent(): Promise<void> {
  const gen = ++startGen
  const player = usePlayer()
  const src = player.source.current
  const s = useSettings().state
  model.value = s.liveSubs.model
  const { lang: l, prompt: p } = langToWhisper(s.liveSubs.sourceLang)
  lang.value = l
  promptText.value = p
  const common = {
    lang: lang.value,
    prompt: promptText.value,
    model: model.value,
    saveSrt: s.liveSubs.saveSrt,
    overwriteOnParamChange: s.liveSubs.overwriteOnParamChange,
    cacheKeyLang: s.liveSubs.sourceLang,
    vadThreshold: s.liveSubs.vad.threshold,
    vadMinSilenceMs: s.liveSubs.vad.minSilenceMs,
  }
  // 外部音源模式：armed 且無影片（有影片時優先走影片路徑）
  if (useAudioSource().armed.value && !src) {
    if (!lang.value) {
      player.notify('請先在設定→即時字幕將來源語言設為明確語言')
      tracks.primary.source = 'off'  // 還原，避免 CC 顯示開但無轉寫
      return
    }
    if (gen !== startGen) return
    await startExternalTranscription(
      model.value,
      lang.value,
      promptText.value ?? '',
      s.liveSubs.vad.threshold,
      s.liveSubs.vad.minSilenceMs,
      s.liveSubs.translateEnabled ? s.liveSubs.translateTo : undefined,
    )
    return
  }
  if (src?.kind === 'remote') {
    // 遠端(YT)：優先從 audio-only 軌抽音訊（不限速）；無則退 muxed playback_url（後端 decode_remote_audio）。跳過 resolveFfIndex；
    // duration / path 用 resolve 權威值與 canonical id（watchUrl，非會過期的 state.path）。
    await invoke('start_transcription', {
      path: src.id,
      sourceKind: 'remote',
      playbackUrl: src.resolved.audioUrl ?? src.resolved.playbackUrl, // 抽音優先用 audio-only 軌（348× 不限速）；無則退 muxed
      headers: src.resolved.httpHeaders,
      durationSec: src.resolved.durationSec,
      ...common,
    })
    return
  }
  const ff = await resolveFfIndex()
  if (gen !== startGen) return // 已被後續 start 或 stop 取代 → 放棄這次（防殭屍/搶跑）
  if (ff == null) { progress.value = { phase: 'error', done: 0, total: null, message: '此檔無音軌' }; return }
  await invoke('start_transcription', {
    path: player.state.path ?? '',
    sourceKind: 'local',
    ffIndex: ff,
    durationSec: player.state.duration ?? 0,
    ...common,
  })
}

function resolveCues(sel: TrackSel): Cue[] {
  if (sel.source === 'live') return liveCues.value
  if (sel.source === 'off') return []
  return files.value.find((f) => f.id === sel.source)?.cues ?? []
}

function enforceSecondaryDep(): void {
  tracks.secondary.source = clampSec(tracks.primary.source, tracks.secondary.source)
}
function selectSource(track: TrackName, source: string): void {
  tracks[track].source = source
  enforceSecondaryDep()
  recordCurrent()
}
/** CC 鈕：暫態切換字幕關/還原來源，不寫 subMemory（不污染 per-video 記憶）。 */
function toggleCc(): void {
  const anyOn = tracks.primary.source !== 'off' || tracks.secondary.source !== 'off'
  if (anyOn) {
    lastSubSnapshot = { primary: tracks.primary.source, secondary: tracks.secondary.source }
    tracks.primary.source = 'off'
    tracks.secondary.source = 'off'
  } else {
    const masterOn = useSettings().state.liveSubs.enabled
    const fileRefs = files.value.map((f) => ({ id: f.id }))
    const restore = pickCcRestore(lastSubSnapshot, fileRefs, masterOn)
    if (!restore) { usePlayer().notify('沒有可顯示的字幕'); return }
    tracks.primary.source = restore.primary
    tracks.secondary.source = restore.secondary
  }
}
const ccActive = computed(() => tracks.primary.source !== 'off' || tracks.secondary.source !== 'off')
function setDelay(track: TrackName, delaySec: number): void { tracks[track].delaySec = delaySec; recordCurrent() }
function addFile(name: string, cues: Cue[], path: string | null, manual: boolean): string {
  const id = `f${++fileCounter}`
  files.value.push({ id, name, cues, path, manual })
  if (manual) recordCurrent()
  return id
}

/** B1 master OFF 時呼叫：把 live 軌設 off（→ watch 停轉寫）。維持 async 供 await/.catch。 */
async function disable(): Promise<void> {
  if (tracks.primary.source === 'live') tracks.primary.source = 'off'
  if (tracks.secondary.source === 'live') tracks.secondary.source = 'off'
  enforceSecondaryDep()
}

/** 清掉殘留進度（provision 對話框關閉時呼叫，避免 dlText banner 殘留一閃）。 */
function clearProgress(): void { progress.value = null }

/** Part B 還原：依 memory[key] 還原 manualFiles + 軌道；失效 manualFile 剔除並持久化。無記憶 → 不還原（N1）。 */
async function restoreFromMemory(key: string, gen: number): Promise<void> {
  const rawEntry = memory[key]
  if (!rawEntry) return // N1：無記憶 → 沿用 baseline（檔案軌已 reset off、live 跨片保留）
  const entry: StoredEntry = coerceStoredEntry(rawEntry) // 防磁碟格式不完整（缺 primary/secondary）觸發 TypeError
  restoreDepth++
  try {
    // 1) 還原 manualFiles（與 Part A 已列檔以 path 去重；讀失敗 → 剔除並記錄持久化）
    const survivors: string[] = []
    for (const p of entry.manualFiles ?? []) {
      if (files.value.some((f) => f.path === p)) { survivors.push(p); continue }
      let text: string
      try { text = await readTextFile(p) } catch { continue } // 檔案被移走/刪除 → 剔除
      if (gen !== fileGen) return
      const name = p.replace(/^.*[\\/]/, '')
      const cues = parseSubtitle(name, text)
      addFile(name, cues, p, true) // restoreDepth>0 → recordCurrent 被壓制
      survivors.push(p)
    }
    if (survivors.length !== (entry.manualFiles ?? []).length) {
      entry.manualFiles = survivors
      const stillEmpty = survivors.length === 0 && entry.primary.source === 'off' && entry.secondary.source === 'off'
      if (stillEmpty) delete memory[key]
      else memory[key] = entry
      scheduleSave()
    }
    // 2) 還原軌道（fileId 由 path 反查；'live' 受 master 控）
    const masterOn = useSettings().state.liveSubs.enabled
    tracks.primary = { source: restoreTrackSource(entry.primary.source, files.value, masterOn), delaySec: entry.primary.delaySec }
    tracks.secondary = { source: restoreTrackSource(entry.secondary.source, files.value, masterOn), delaySec: entry.secondary.delaySec }
    enforceSecondaryDep()
  } finally {
    restoreDepth--
  }
}

/** 換檔：清空 files；指向已清除 file 的軌設 off；Part A 探索同資料夾字幕（僅 local）；live 軌保留並對新檔重啟。 */
async function onFileChanged(): Promise<void> {
  const src = usePlayer().source.current
  const id = src?.id ?? ''
  if (id && id === lastFileId) return   // 同片（畫質 reload / path→null→同值）→ 略過整檔重置（MAJ-3）
  lastFileId = id
  lastSubSnapshot = null                // 換片清 CC 快照
  const gen = ++fileGen
  const path = id
  files.value = []
  for (const t of ['primary', 'secondary'] as const) {
    if (tracks[t].source !== 'live' && tracks[t].source !== 'off') tracks[t].source = 'off'
  }
  // Part A：同資料夾 sidecar 自動列出（只列出、不自動選取）。remote(URL) 無資料夾概念 → 跳過。
  if (src?.kind === 'local' && path) {
    let list: SidecarSub[] = []
    try { list = await listSidecarSubs(path) } catch { list = [] }
    if (gen !== fileGen) return
    for (const s of list) {
      let text: string
      try { text = await readTextFile(s.path) } catch { continue }
      if (gen !== fileGen) return
      const cues = parseSubtitle(s.name, text)
      addFile(s.name, cues, s.path, false)
    }
  }
  // Part B：per-video 記憶還原（接在 Part A 之後）。
  if (path) {
    await ensureWired()
    if (gen !== fileGen) return
    await restoreFromMemory(normKey(path), gen)
    if (gen !== fileGen) return
  }
  // 還原後顯式啟動 live（不靠 watch(liveNeeded) transition；前片已 live 時 source='live' 是 no-op、watcher 不會 fire）。
  if (liveNeeded.value) await startForCurrent()
}

const activeText = (track: TrackName, t: number): string =>
  selectCueAt(resolveCues(tracks[track]), t - tracks[track].delaySec)?.sourceText ?? ''
const isTranscribing = (track: TrackName, t: number): boolean =>
  tracks[track].source === 'live' && t > frontierSec.value && !activeText(track, t)

/** 純函式：依播放狀態與 armed 狀態決定 CC 鈕三態。
 *  playing=true → 'file'（控制片字幕）
 *  idle + armed  → 'external'（外部辨識 toggle）
 *  idle 未 armed → 'disabled'
 */
export function ccMode(playing: boolean, armed: boolean): 'file' | 'external' | 'disabled' {
  if (playing) return 'file'
  if (armed) return 'external'
  return 'disabled'
}

export function useSubtitles() {
  ensureWired()
  return {
    enabled: readonly(liveNeeded),
    progress: readonly(progress),
    files: readonly(files),
    tracks,
    selectSource,
    toggleCc,
    ccActive: readonly(ccActive),
    setDelay,
    addFile,
    disable,
    onFileChanged,
    activeText,
    isTranscribing,
    clearProgress,
    flushMemory,
    noClock: readonly(noClock),
    liveCues: readonly(liveCues),
    liveInterim: readonly(liveInterim),
  }
}
