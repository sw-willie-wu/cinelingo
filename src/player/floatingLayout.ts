// height 不持久化：由行數即時算（見 captionBarHeight），故 Rect 只存位置與寬。
export interface Rect { x: number; y: number; width: number }
export interface Mon { x: number; y: number; width: number; height: number }

const DEFAULT_WIDTH_FRACTION = 0.7
const BOTTOM_MARGIN = 60          // 字幕條底邊距螢幕下緣 px
const LINE_HEIGHT = 1.5           // 每行高 = 字級 × 此倍率（含行距、上下呼吸空間，避免切到字）
const BAR_PADDING = 16            // 上下內距總和
const MIN_VISIBLE_FRACTION = 0.5  // rect 至少這比例面積落在某螢幕才算可見
const FALLBACK_MONITOR: Mon = { x: 0, y: 0, width: 1280, height: 720 }  // 無螢幕資訊時退路

/**
 * 由「行數 × 字級」估字幕條視窗高（px）。font-aware：字級大（如 4K 全螢幕等比的 56px）時條才夠高、不切字。
 * fontPx = 浮動條實際渲染字級（依螢幕高 scaleFontPx 算出）。
 */
export function captionBarHeight(lines: number, fontPx: number): number {
  return Math.round(Math.max(1, lines) * Math.max(1, fontPx) * LINE_HEIGHT + BAR_PADDING)
}

/** 主螢幕（取 monitors[0]）底部置中 + 預設寬。無螢幕 → 安全退路。 */
export function defaultFloatingRect(height: number, monitors: Mon[]): Rect {
  const m = monitors[0] ?? FALLBACK_MONITOR
  const width = Math.round(m.width * DEFAULT_WIDTH_FRACTION)
  return {
    x: m.x + Math.round((m.width - width) / 2),
    y: m.y + Math.max(0, m.height - height - BOTTOM_MARGIN),
    width,
  }
}

/** rect 與某螢幕的交集面積比（相對 rect 面積）。 */
function visibleFraction(r: Rect, height: number, m: Mon): number {
  const ix = Math.max(r.x, m.x)
  const iy = Math.max(r.y, m.y)
  const ix2 = Math.min(r.x + r.width, m.x + m.width)
  const iy2 = Math.min(r.y + height, m.y + m.height)
  const iw = Math.max(0, ix2 - ix)
  const ih = Math.max(0, iy2 - iy)
  const area = r.width * height
  return area > 0 ? (iw * ih) / area : 0
}

/**
 * 還原持久化位置前的可視性驗證。height = 由行數即時算（非持久值）。
 * 若 rect 與所有螢幕的最大可見比 < MIN_VISIBLE_FRACTION → 回退主螢幕底部置中 + 預設寬。
 */
export function clampToVisible(saved: Rect, height: number, monitors: Mon[]): Rect {
  if (monitors.length === 0) return defaultFloatingRect(height, monitors)
  const best = Math.max(...monitors.map((m) => visibleFraction(saved, height, m)))
  if (best >= MIN_VISIBLE_FRACTION) return { x: saved.x, y: saved.y, width: saved.width }
  return defaultFloatingRect(height, monitors)
}
