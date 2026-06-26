import { describe, it, expect } from 'vitest'
import { shouldHide } from '../player/autohide'

describe('shouldHide', () => {
  it('有檔 + 指標不在列 → 隱藏(暫停也會隱藏)', () => {
    expect(shouldHide({ hasFile: true, pointerOverBar: false })).toBe(true)
  })
  it('無檔時不隱藏', () => {
    expect(shouldHide({ hasFile: false, pointerOverBar: false })).toBe(false)
  })
  it('指標在控制列上時不隱藏', () => {
    expect(shouldHide({ hasFile: true, pointerOverBar: true })).toBe(false)
  })
})
