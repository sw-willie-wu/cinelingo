import { ref } from 'vue'
import { useSettings } from './useSettings'
import { usePlayer } from './usePlayer'
import { buildAf, buildEqBands } from './audioAf'
import { setAf, setAudioDelay } from '../mpv'
import { writeEntry } from './playbackMemory'

// 音訊延遲為 per-video：以本地 ref 持有當前片的值（換片由 playbackMemory.applyForCurrent 設定）。
const audioDelayMs = ref(0)

export function setAudioDelayCurrent(ms: number, persist = true): void {
  const clamped = Math.max(-2000, Math.min(2000, Math.round(ms / 50) * 50))
  audioDelayMs.value = clamped
  setAudioDelay(clamped / 1000).catch((e) => console.error('[audio] setAudioDelay', e))
  if (persist) {
    const id = usePlayer().source.current?.id
    if (id) writeEntry(id, { audioDelaySec: clamped / 1000 }).catch(() => {})
  }
}

export function useAudioAdjust() {
  const s = useSettings().state
  function applyAf(): void {
    setAf(buildAf({ eq: { enabled: s.audio.eq.enabled, bands: s.audio.eq.bands }, normalize: s.audio.normalize }))
      .catch((e) => console.error('[audio] setAf', e))
  }
  function setEqEnabled(on: boolean): void { s.audio.eq.enabled = on; applyAf() }
  function setBand(i: number, gain: number): void {
    s.audio.eq.bands[i] = Math.max(-12, Math.min(12, Math.round(gain)))
    s.audio.eq.preset = 'custom'
    applyAf()
  }
  function setPreset(preset: string): void {
    s.audio.eq.preset = preset
    s.audio.eq.bands = buildEqBands(preset)
    applyAf()
  }
  function setNormalize(on: boolean): void { s.audio.normalize = on; applyAf() }
  /** 啟動套用。音訊延遲不在此（per-video，由換片流程套）。 */
  async function applyFromSettings(): Promise<void> { applyAf() }
  return {
    eq: s.audio.eq, normalize: () => s.audio.normalize, audioDelayMs,
    setEqEnabled, setBand, setPreset, setNormalize, setAudioDelay: setAudioDelayCurrent, applyFromSettings,
  }
}
