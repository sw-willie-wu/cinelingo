import { describe, it, expect } from 'vitest'
import { clampSecondaryToPrimary } from '../player/ccRestore'

describe('第二字幕相依落地一致', () => {
  it('面板選 primary off → secondary clamp 成 off', () => {
    const primary = 'off'
    let secondary = 'f2'
    secondary = clampSecondaryToPrimary(primary, secondary)
    expect(secondary).toBe('off')
  })
  it('primary 非 off → secondary 保留', () => {
    expect(clampSecondaryToPrimary('live', 'f3')).toBe('f3')
  })
})
