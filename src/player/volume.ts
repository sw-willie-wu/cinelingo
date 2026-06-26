// 夾在 0–max。UI 上限刻意設 100(非 mpv --volume-max 預設 130;產品選擇)。
export function clampVolume(v: number, max = 100): number {
  if (!Number.isFinite(v)) return 0
  return Math.min(max, Math.max(0, v))
}
