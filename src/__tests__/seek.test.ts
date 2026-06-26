import { describe, it, expect } from 'vitest'
import { fractionToTime, timeToFraction, bufferedFraction } from '../player/seek'

describe('fractionToTime', () => {
  it('0.5 → 一半秒數', () => { expect(fractionToTime(0.5, 100)).toBe(50) })
  it('fraction clamp 到 [0,1]', () => {
    expect(fractionToTime(-1, 100)).toBe(0)
    expect(fractionToTime(2, 100)).toBe(100)
  })
  it('duration 為 null/0/負 → 0', () => {
    expect(fractionToTime(0.5, null)).toBe(0)
    expect(fractionToTime(0.5, 0)).toBe(0)
    expect(fractionToTime(0.5, -10)).toBe(0)
  })
})

describe('timeToFraction', () => {
  it('一半秒數 → 0.5', () => { expect(timeToFraction(50, 100)).toBe(0.5) })
  it('結果 clamp 到 [0,1]', () => {
    expect(timeToFraction(-5, 100)).toBe(0)
    expect(timeToFraction(150, 100)).toBe(1)
  })
  it('duration 無效或 t 為 null → 0', () => {
    expect(timeToFraction(50, null)).toBe(0)
    expect(timeToFraction(50, 0)).toBe(0)
    expect(timeToFraction(null, 100)).toBe(0)
  })
})

describe('bufferedFraction', () => {
  it('正常比例', () => { expect(bufferedFraction(30, 120)).toBe(0.25) })
  it('cacheTime null → 0', () => { expect(bufferedFraction(null, 120)).toBe(0) })
  it('duration null/0 → 0', () => { expect(bufferedFraction(30, null)).toBe(0); expect(bufferedFraction(30, 0)).toBe(0) })
  it('超界 clamp 到 1', () => { expect(bufferedFraction(200, 120)).toBe(1) })
  it('負值 → 0', () => { expect(bufferedFraction(-5, 120)).toBe(0) })
})
