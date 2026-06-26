function clamp(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v))
}

function validDuration(d: number | null): d is number {
  return d != null && Number.isFinite(d) && d > 0
}

// fraction(0–1) → 對應秒數,clamp 至 [0, duration]
export function fractionToTime(fraction: number, duration: number | null): number {
  if (!validDuration(duration)) return 0
  return clamp(clamp(fraction, 0, 1) * duration, 0, duration)
}

// 秒 → fraction(0–1);duration 無效或 t 為 null → 0
export function timeToFraction(t: number | null, duration: number | null): number {
  if (!validDuration(duration)) return 0
  if (t == null || !Number.isFinite(t)) return 0
  return clamp(t / duration, 0, 1)
}

// 已緩衝比例：demuxer-cache-time / duration，clamp 0–1。無效輸入 → 0。
export function bufferedFraction(cacheTime: number | null, duration: number | null): number {
  if (!validDuration(duration)) return 0
  if (cacheTime == null || !Number.isFinite(cacheTime)) return 0
  return clamp(cacheTime / duration, 0, 1)
}
