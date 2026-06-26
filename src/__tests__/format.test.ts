import { describe, it, expect } from 'vitest'
import { formatClock, baseName, formatStatus, displayTitle } from '../player/format'

describe('formatClock', () => {
  it('null 顯示 --:--', () => { expect(formatClock(null)).toBe('--:--') })
  it('非有限值顯示 --:--', () => { expect(formatClock(NaN)).toBe('--:--') })
  it('負值顯示 --:--', () => { expect(formatClock(-3)).toBe('--:--') })
  it('一分鐘內補零', () => { expect(formatClock(12)).toBe('00:12') })
  it('分秒補零', () => { expect(formatClock(75)).toBe('01:15') })
  it('滿一小時顯示 H:MM:SS', () => { expect(formatClock(3661)).toBe('1:01:01') })
  it('無條件捨去到秒', () => { expect(formatClock(12.9)).toBe('00:12') })
})

describe('baseName', () => {
  it('取 Windows 路徑檔名', () => { expect(baseName('C:\\v\\movie.mkv')).toBe('movie.mkv') })
  it('取 POSIX 路徑檔名', () => { expect(baseName('/x/clip.mp4')).toBe('clip.mp4') })
  it('無分隔回原字串', () => { expect(baseName('a.mp4')).toBe('a.mp4') })
})

describe('formatStatus', () => {
  it('無檔案時顯示提示', () => {
    expect(formatStatus({ pause: null, path: null, timePos: null })).toBe('尚未載入檔案')
  })
  it('播放中顯示檔名與時間', () => {
    expect(formatStatus({ pause: false, path: 'C:/v/movie.mkv', timePos: 12 }))
      .toBe('▶ 播放中 — movie.mkv (00:12)')
  })
  it('暫停顯示暫停標記', () => {
    expect(formatStatus({ pause: true, path: '/x/clip.mp4', timePos: 75 }))
      .toBe('⏸ 已暫停 — clip.mp4 (01:15)')
  })
})

describe('displayTitle', () => {
  it('remote title 優先', () => {
    expect(displayTitle('醉', '/x/video.mp4')).toBe('醉')
  })
  it('無 remote title → baseName(path)', () => {
    expect(displayTitle(null, 'C:/movies/film.mkv')).toBe('film.mkv')
  })
  it('空 title 視同無 → baseName', () => {
    expect(displayTitle('', 'C:/a.mkv')).toBe('a.mkv')
  })
  it('無 title 無 path → Cinelingo', () => {
    expect(displayTitle(null, null)).toBe('Cinelingo')
  })
})
