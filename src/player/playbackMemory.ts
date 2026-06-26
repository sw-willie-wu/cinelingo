import { loadPlaybackMemory, savePlaybackMemory } from './backend'
import { normKey } from './subMemory'
import { applySpeed } from './useSpeed'
import { setAudioDelayCurrent } from './useAudioAdjust'

export interface PlaybackEntry { speed?: number; audioDelaySec?: number }

export function coercePlaybackEntry(raw: unknown): PlaybackEntry {
  const r = raw !== null && typeof raw === 'object' ? (raw as Record<string, unknown>) : {}
  const out: PlaybackEntry = {}
  if (typeof r.speed === 'number' && isFinite(r.speed)) out.speed = r.speed
  if (typeof r.audioDelaySec === 'number' && isFinite(r.audioDelaySec)) out.audioDelaySec = r.audioDelaySec
  return out
}
export function mergeEntry(a: PlaybackEntry, b: PlaybackEntry): PlaybackEntry {
  return { ...a, ...b }
}
export function isDefaultEntry(e: PlaybackEntry): boolean {
  return (e.speed === undefined || e.speed === 1) && (e.audioDelaySec === undefined || e.audioDelaySec === 0)
}

let memory: Record<string, unknown> = {}
let loaded: Promise<void> | null = null
let saveTimer: ReturnType<typeof setTimeout> | null = null

function ensureLoaded(): Promise<void> {
  if (!loaded) loaded = (async () => { try { memory = await loadPlaybackMemory() } catch { memory = {} } })()
  return loaded
}
function scheduleSave(): void {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(() => { saveTimer = null; savePlaybackMemory(memory).catch((e) => console.warn('[playbackMemory] save failed', e)) }, 300)
}

export async function readEntry(canonicalId: string): Promise<PlaybackEntry> {
  await ensureLoaded()
  return coercePlaybackEntry(memory[normKey(canonicalId)])
}
/** 更新某片的 speed/audioDelay；回預設則刪 key。debounce 落地。 */
export async function writeEntry(canonicalId: string, patch: PlaybackEntry): Promise<void> {
  await ensureLoaded()
  const key = normKey(canonicalId)
  const next = mergeEntry(coercePlaybackEntry(memory[key]), patch)
  if (isDefaultEntry(next)) delete memory[key]
  else memory[key] = next
  scheduleSave()
}

let applyLastId = ''
let applyGen = 0
/** 換片套用 per-video speed/audio-delay；同片（畫質 reload）略過。供 App.vue path watch 呼叫。 */
export async function applyForCurrent(canonicalId: string): Promise<void> {
  if (canonicalId && canonicalId === applyLastId) return   // 同片 → 不重置
  applyLastId = canonicalId
  const gen = ++applyGen
  const entry = canonicalId ? await readEntry(canonicalId) : {}
  if (gen !== applyGen) return                              // 被新檔取代 → 放棄 stale 套用
  applySpeed(entry.speed ?? 1, false)                      // persist=false：還原不重複落地
  setAudioDelayCurrent((entry.audioDelaySec ?? 0) * 1000, false)
}
