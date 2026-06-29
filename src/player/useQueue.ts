import { reactive } from 'vue'
import { usePlayer } from './usePlayer'
import { useRecent } from './useRecent'
import { resolveRemote, stopExternalTranscription } from './backend'
import { useSettings } from './useSettings'
import { REMOTE_TITLE_LOADING, type QueueItem } from './queueTypes'
import { useAudioSource } from './useAudioSource'

const items = reactive<QueueItem[]>([])
const state = reactive<{ index: number }>({ index: -1 })

let playGen = 0              // 意圖計數；playAt 進入即 ++（競態守衛）
let liveGen = 0             // 已交給 mpv 且「現正生效」的那次載入之 gen
let liveItem: QueueItem | null = null   // liveGen 對應項目（事件關聯：記錄/自動下一支）
let liveEntryId: number | null = null   // start-file 捕捉的 mpv playlist_entry_id（事件關聯主鍵）
let expectStartFile = false
let ended = false                        // 佇列已播到尾並停住（eof/error 無下一支）→ 之後 append 新項自動接續播
let wired = false
let pendingReload: { gen: number; seekSec: number; restorePause: boolean } | null = null

async function playAt(i: number): Promise<void> {
  if (i < 0 || i >= items.length) return
  const player = usePlayer()
  const gen = ++playGen
  if (pendingReload && pendingReload.gen !== playGen) pendingReload = null
  ended = false                          // 開始一次播放 → 清除「已播完」狀態
  state.index = i
  const item = items[i]
  // 轉場：若外部音源已 armed，先解除並停擷取轉寫（fire-and-forget；後端 no-op 若未在轉寫）
  const audioSource = useAudioSource()
  if (audioSource.armed.value) {
    void audioSource.disarm()
    void stopExternalTranscription()
  }
  let ok = false
  if (item.kind === 'remote') {
    player.setResolving(true)
    let r: Awaited<ReturnType<typeof resolveRemote>> | null = null
    try { r = await resolveRemote(item.id) } catch { /* notify below */ }
    player.setResolving(false)
    if (gen !== playGen) return                                       // 被取代 → 不碰 mpv/source
    if (!r) { player.notify('無法解析此連結（不支援的網站或無法存取）'); await next(); return } // spec §8
    if (r.title) item.title = r.title                                 // 用解析出的真實標題回填（單片貼上原為 URL）→ 佇列/最近即時顯示
    else if (item.title === REMOTE_TITLE_LOADING) item.title = item.id // resolve 無標題：別讓播放中項卡載入哨兵（recent 也不會存到哨兵）
    ok = await player.playResolvedRemote(item.id, r)                  // isLive/no-url 由 playResolvedRemote 自行 notify
  } else {
    if (gen !== playGen) return
    ok = await player.loadPath(item.id)                              // 同步/命令錯回 false；本地缺檔走 end-file{error}
  }
  if (gen !== playGen) return
  if (!ok) { await next(); return }                                  // 已 notify（resolve/isLive/no-url）；本地命令錯靜默 skip
  // 交付成功 → 標記現正生效。liveEntryId 重置為 null：等本次載入的 start-file 重新捕捉；
  // 若因 start-file 與 loadfile-ack 亂序而漏捕（spec §9.1b），entryId 守衛退化成 null 短路 →
  // 只靠 playGen===liveGen（安全、正確），不會用殘留舊 entryId 誤拒有效 eof。
  liveGen = gen; liveItem = item; liveEntryId = null; expectStartFile = true
}

async function next(): Promise<void> {
  if (state.index + 1 < items.length) await playAt(state.index + 1)
}

async function prev(): Promise<void> {
  if (state.index - 1 >= 0) await playAt(state.index - 1)
}

async function enqueueItems(
  newItems: QueueItem[],
  opts?: { startOffset?: number; interrupt?: boolean; noAutoplay?: boolean },
): Promise<void> {
  if (newItems.length === 0) return
  const player = usePlayer()
  const wasEmpty = items.length === 0
  const base = items.length
  items.push(...newItems)                  // 一律 append（Q1：清單也 append、不取代）
  const off = opts?.startOffset ?? 0
  let target: number | null = null
  // 空佇列 / 已播完(eof) / 目前沒在播(停止後 idle) 且非 noAutoplay → 自動播進入點。
  // isIdle 補掉「按停止後 ended 不會被設(end-file reason=stop 被忽略) → 新拖入只 append 不播」的洞。
  if ((wasEmpty || ended || player.isIdle.value) && !opts?.noAutoplay) target = base + off
  else if (opts?.interrupt) target = base + off  // 非空＋interrupt：跳到新加入項（最近點擊）
  // else（非空、播放中、不 interrupt；或 noAutoplay）→ 只 append、不打斷
  if (target != null) await playAt(target)
}

function remove(i: number): void {
  if (i < 0 || i >= items.length) return
  const wasCurrent = i === state.index
  items.splice(i, 1)
  if (i < state.index) state.index--
  if (wasCurrent) {
    if (items.length === 0) { state.index = -1; void usePlayer().closeMedia() }  // 刪掉正在播的唯一項 → 停止播放回首頁
    else void playAt(Math.min(i, items.length - 1))   // 以「移除後同位置項」遞補
  }
}

function move(from: number, to: number): void {
  if (from < 0 || from >= items.length || to < 0 || to >= items.length) return
  const [it] = items.splice(from, 1)
  items.splice(to, 0, it)
  if (state.index === from) state.index = to
  else if (from < state.index && to >= state.index) state.index--
  else if (from > state.index && to <= state.index) state.index++
}

function clear(): void {
  items.splice(0, items.length)
  state.index = -1
  liveGen = 0; liveItem = null; liveEntryId = null; expectStartFile = false; ended = false
  // ⚠️ 不重置 playGen：單調世代計數，重置成 0 會重用號碼 → 清空後重貼時 gen 撞號，
  // 先前未完成的 stale resolve 會通過守衛覆蓋 source.current/載入已清掉的影片。
}

// 背景抓到的 remote 標題回填到「仍在載入」的同 id 項。透過 items.find 取得 reactive proxy 再 mutate
// → 觸發 UI 更新（usePasteUrl 持有的是 push 前的原始物件，直接改它不具反應性）。
// 判斷式含「仍是哨兵」守衛：不覆寫 playAt 已解析的真標題；重複貼同片時每次抓回各補一個尚未補的。
function backfillTitle(id: string, title: string): void {
  const it = items.find((x) => x.id === id && x.title === REMOTE_TITLE_LOADING)
  if (it) it.title = title
}

async function reloadCurrentQuality(quality: 'auto' | number): Promise<void> {
  const player = usePlayer()
  const cur = player.source.current
  if (!cur || cur.kind !== 'remote') return
  useSettings().state.youtube.quality = quality
  const gen = ++playGen
  liveGen = gen; liveEntryId = null; expectStartFile = true
  const pos = player.state.timePos ?? 0
  const dur = player.state.duration ?? 0
  pendingReload = { gen, seekSec: pos, restorePause: player.state.pause === true }
  player.beginQualitySwitch(pos, dur > 0 ? pos / dur : 0, dur)  // 凍結進度條/時間 + spinner
  await player.reloadQualityFmt(quality)
}

function ensureWired(): void {
  if (wired) return
  wired = true
  const player = usePlayer()
  const recent = useRecent()
  player.onStartFile((entryId: number) => {
    if (expectStartFile) { liveEntryId = entryId; expectStartFile = false }
  })
  player.onFileLoaded(() => {
    if (pendingReload && playGen === pendingReload.gen) {
      const { seekSec, restorePause } = pendingReload
      pendingReload = null
      void player.seekToImmediate(seekSec)   // 不受 duration 守衛擋（reload 當下 duration 常為 null）
      if (restorePause) void player.setPauseTrue()
      setTimeout(() => player.endQualitySwitch(), 800)  // seek 落地後放開凍結（此時 time-pos 已回到原位）
      return  // 畫質 reload：跳過 recent.record（避免重複「最近」）
    }
    if (liveItem) recent.record(liveItem) // file-loaded 只對成功開啟的檔發 → 失敗檔不入最近
  })
  player.onEndFile((reason: string, entryId: number) => {
    if (playGen !== liveGen) return                          // 有更新 playAt 在飛 → 此 end-file 屬被替換的舊檔，忽略
    if (liveEntryId !== null && entryId !== liveEntryId) return  // entry-id 關聯（事件無 identity 的補強）
    if (reason !== 'eof' && reason !== 'error') return       // stop/redirect/quit → 忽略（多為我方換檔/關閉）
    if (reason === 'error') player.notify('此項無法播放，已跳過') // 本地缺檔/壞檔
    if (state.index + 1 < items.length) void next()
    else ended = true                                        // 佇列播完、停在最後一幀 → 之後 append 新項自動接續
  })
}

export function useQueue() {
  ensureWired()
  return { items, state, enqueueItems, playAt, next, prev, remove, move, clear, backfillTitle, reloadCurrentQuality }
}
