// 給 formatStatus 用的最小狀態(不耦合 MpvState,故 MpvState 擴充欄位時不會破這裡的測試)
export interface ClockStatus {
  path: string | null
  pause: boolean | null
  timePos: number | null
}

export function baseName(p: string): string {
  const parts = p.split(/[\\/]/)
  return parts[parts.length - 1] || p
}

/** titlebar 顯示名：遠端有 title 用 title；否則 baseName(path)；無 path → 'Cinelingo'。 */
export function displayTitle(remoteTitle: string | null, statePath: string | null): string {
  if (remoteTitle) return remoteTitle
  return statePath ? baseName(statePath) : 'Cinelingo'
}

function pad2(n: number): string {
  return String(n).padStart(2, '0')
}

// 秒 → "M:SS"(或 "H:MM:SS");null/非有限/負 → "--:--"
export function formatClock(sec: number | null): string {
  if (sec == null || !Number.isFinite(sec) || sec < 0) return '--:--'
  const total = Math.floor(sec)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  return h > 0 ? `${h}:${pad2(m)}:${pad2(s)}` : `${pad2(m)}:${pad2(s)}`
}

export function formatStatus(s: ClockStatus): string {
  if (!s.path) return '尚未載入檔案'
  const mark = s.pause ? '⏸ 已暫停' : '▶ 播放中'
  return `${mark} — ${baseName(s.path)} (${formatClock(s.timePos)})`
}
