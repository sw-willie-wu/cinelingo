import { describe, it, expect } from 'vitest'
import { clampToVisible, captionBarHeight, type Rect, type Mon } from '../player/floatingLayout'

const mon = (x: number, y: number, w: number, h: number): Mon => ({ x, y, width: w, height: h })
const M = [mon(0, 0, 1920, 1080)]

describe('clampToVisible', () => {
  it('keeps a fully-visible rect', () => {
    const r: Rect = { x: 200, y: 900, width: 1000 }
    expect(clampToVisible(r, 120, M)).toEqual({ x: 200, y: 900, width: 1000 })
  })
  it('falls back to bottom-center when fully off-screen', () => {
    const r: Rect = { x: 5000, y: 5000, width: 1000 }
    const out = clampToVisible(r, 120, M)
    expect(out.width).toBe(Math.round(1920 * 0.7))
    expect(out.x).toBe(Math.round((1920 - out.width) / 2))
  })
  it('falls back when bottom edge below screen by more than height (only title sliver visible)', () => {
    const r: Rect = { x: 200, y: 1070, width: 1000 } // y+120 = 1190, 只露 10px
    const out = clampToVisible(r, 120, M)
    expect(out.x).toBe(Math.round((1920 - out.width) / 2)) // 視為不可見 → fallback
  })
  it('falls back when no monitors', () => {
    const out = clampToVisible({ x: 0, y: 0, width: 800 }, 120, [])
    expect(out.width).toBeGreaterThan(0)
  })
})

describe('captionBarHeight', () => {
  it('grows with line count and font size', () => {
    expect(captionBarHeight(2, 28)).toBeLessThan(captionBarHeight(5, 28))   // 行數越多越高
    expect(captionBarHeight(3, 28)).toBeLessThan(captionBarHeight(3, 56))   // 字級越大越高（避免切字）
  })
})
