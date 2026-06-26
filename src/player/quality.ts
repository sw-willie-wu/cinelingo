import type { VideoFormat } from './backend'

export type QualityPref = 'auto' | number

// videos 假設已降冪 by height。auto → 最高；具體值 → 命中、否則 ≤ 該值最高、再否則（全高於）取最低可用（最接近上限）。空 → null。
export function pickVideoFormat(videos: readonly VideoFormat[], pref: QualityPref): VideoFormat | null {
  if (videos.length === 0) return null
  if (pref === 'auto') return videos[0]
  const exact = videos.find((v) => v.height === pref)
  if (exact) return exact
  const below = videos.find((v) => v.height <= pref) // 降冪 → 第一個 ≤ 即最高的 ≤
  return below ?? videos[videos.length - 1] // 全高於 → 末項（最低可用，最接近 pref）
}

/** 'auto' 的上限＝螢幕實體像素高度（CSS 高 × DPR）。非瀏覽器環境退回 1080。 */
export function autoCapHeight(): number {
  if (typeof window === 'undefined' || !window.screen) return 1080
  return Math.round(window.screen.height * (window.devicePixelRatio || 1))
}

/** 依設定挑影軌：'auto' → 不超過螢幕實體解析度的最高（省頻寬）；具體數值 → 原樣（手動可超螢幕，2K/4K 降採樣）。 */
export function pickForPref(videos: readonly VideoFormat[], pref: QualityPref): VideoFormat | null {
  return pickVideoFormat(videos, pref === 'auto' ? autoCapHeight() : pref)
}
