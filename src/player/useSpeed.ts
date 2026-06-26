import { ref } from 'vue'
import { usePlayer } from './usePlayer'
import { setSpeed as mpvSetSpeed } from '../mpv'
import { writeEntry } from './playbackMemory'

const speed = ref(1)
export const SPEED_PRESETS = [0.5, 0.75, 1, 1.25, 1.5, 1.75, 2] as const

/** 設定速度（夾 0.25..4）。persist=false 供換片還原時用（不重複落地）。 */
export function applySpeed(v: number, persist = true): void {
  const clamped = Math.max(0.25, Math.min(4, Math.round(v * 100) / 100))
  speed.value = clamped
  mpvSetSpeed(clamped).catch((e) => console.error('[speed] setSpeed', e))
  if (persist) {
    const id = usePlayer().source.current?.id
    if (id) writeEntry(id, { speed: clamped }).catch(() => {})
  }
}

export function useSpeed() {
  return { speed, presets: SPEED_PRESETS, setSpeed: applySpeed }
}
