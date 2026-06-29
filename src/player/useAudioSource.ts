import { ref, readonly } from 'vue'
import type { Ref } from 'vue'
import { listen } from '@tauri-apps/api/event'
import type { AudioSources } from './backend'
import { listAudioSources, armAudioSource, disarmAudioSource } from './backend'
import type { PersistedSource } from './settings'
import { useSettings } from './useSettings'

const LEVEL_GAIN = 6
export function normalizeLevel(rms: number): number {
  return Math.max(0, Math.min(1, rms * LEVEL_GAIN))
}

// 錄音檔名（時間戳，本地時間）；未開啟錄製 → null（後端不錄）。
function recordingName(): string | null {
  if (!useSettings().state.capture.recordAudio) return null
  const d = new Date()
  const p = (n: number) => String(n).padStart(2, '0')
  return `capture-${d.getFullYear()}${p(d.getMonth() + 1)}${p(d.getDate())}-${p(d.getHours())}${p(d.getMinutes())}${p(d.getSeconds())}.wav`
}

// Sent to backend — matches Rust AudioSource enum serde
export type AudioSource =
  | { kind: 'system' }
  | { kind: 'process'; pid: number }
  | { kind: 'inputDevice'; id: string }

// What panel returns — process has both name (for persistence) + pid (for backend)
export type PanelSelection =
  | { kind: 'system' }
  | { kind: 'process'; name: string; pid: number }
  | { kind: 'inputDevice'; id: string }

/** Resolve a persisted source to a backend-ready AudioSource using the live process list.
 *  process: look up by name in live.processes → get pid; if not found → fall back to system.
 */
export function resolveSource(persisted: PersistedSource, live: AudioSources): AudioSource {
  if (persisted.kind === 'system') return { kind: 'system' }
  if (persisted.kind === 'inputDevice') return { kind: 'inputDevice', id: persisted.id }
  // kind === 'process'
  const found = live.processes.find((p) => p.name === persisted.name)
  if (found) return { kind: 'process', pid: found.pid }
  return { kind: 'system' }
}

// Module-level singleton state
const armed = ref(false)
const current = ref<PanelSelection | null>(null)
const sources = ref<AudioSources>({ processes: [], inputDevices: [] })
const level = ref(0)

// Subscribe to backend capture-level events (emitted ~80ms while armed)
;(async () => {
  try {
    await listen<number>('capture-level', (e) => { level.value = normalizeLevel(e.payload) })
  } catch (e) {
    console.warn('[useAudioSource] listen capture-level failed', e)
  }
})()

async function refresh(): Promise<void> {
  try {
    sources.value = await listAudioSources()
  } catch (e) {
    console.warn('[useAudioSource] listAudioSources failed', e)
  }
}

async function arm(sel: PanelSelection): Promise<void> {
  // Convert PanelSelection → AudioSource (process: drop name, keep pid)
  let source: AudioSource
  if (sel.kind === 'process') {
    source = { kind: 'process', pid: sel.pid }
  } else if (sel.kind === 'inputDevice') {
    source = { kind: 'inputDevice', id: sel.id }
  } else {
    source = { kind: 'system' }
  }

  await armAudioSource(source, recordingName())

  // Persist: for process keep name, drop pid
  let persisted: PersistedSource
  if (sel.kind === 'process') {
    persisted = { kind: 'process', name: sel.name }
  } else if (sel.kind === 'inputDevice') {
    persisted = { kind: 'inputDevice', id: sel.id }
  } else {
    persisted = { kind: 'system' }
  }
  useSettings().state.liveSubs.audioSource = persisted

  armed.value = true
  current.value = sel
}

async function disarm(): Promise<void> {
  await disarmAudioSource()
  armed.value = false
}

export function useAudioSource(): {
  armed: Readonly<Ref<boolean>>
  current: Readonly<Ref<PanelSelection | null>>
  sources: Ref<AudioSources>
  level: Readonly<Ref<number>>
  refresh(): Promise<void>
  arm(sel: PanelSelection): Promise<void>
  disarm(): Promise<void>
} {
  return {
    armed: readonly(armed),
    current: readonly(current),
    sources,
    level: readonly(level),
    refresh,
    arm,
    disarm,
  }
}
