import { describe, it, expect } from 'vitest'
import { clampVolume } from '../player/volume'

describe('clampVolume', () => {
  it('區間內原值', () => { expect(clampVolume(50)).toBe(50) })
  it('上限預設 100', () => { expect(clampVolume(130)).toBe(100) })
  it('下限 0', () => { expect(clampVolume(-5)).toBe(0) })
  it('可自訂上限', () => { expect(clampVolume(130, 130)).toBe(130) })
  it('非有限值 → 0', () => { expect(clampVolume(NaN)).toBe(0) })
})
