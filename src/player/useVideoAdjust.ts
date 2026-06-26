import { useSettings } from './useSettings'
import { setImageProp, setDeband, type ImageProp } from '../mpv'

const IMAGE_PROPS: ImageProp[] = ['brightness', 'contrast', 'saturation', 'gamma', 'hue']

export function useVideoAdjust() {
  const s = useSettings().state
  function setProp(name: ImageProp, v: number): void {
    const clamped = Math.max(-100, Math.min(100, Math.round(v)))
    s.video[name] = clamped
    setImageProp(name, clamped).catch((e) => console.error('[videoAdjust] setImageProp', e))
  }
  function setDebandOn(on: boolean): void {
    s.video.deband = on
    setDeband(on).catch((e) => console.error('[videoAdjust] setDeband', e))
  }
  function reset(): void {
    IMAGE_PROPS.forEach((p) => setProp(p, 0))
    setDebandOn(false)
  }
  /** 啟動套用（startMpv 後呼叫一次；mpv 屬性跨 loadfile 持續）。 */
  async function applyFromSettings(): Promise<void> {
    for (const p of IMAGE_PROPS) await setImageProp(p, s.video[p])
    await setDeband(s.video.deband)
  }
  return { state: s.video, props: IMAGE_PROPS, setProp, setDebandOn, reset, applyFromSettings }
}
