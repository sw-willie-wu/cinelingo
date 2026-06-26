import { describe, it, expect } from 'vitest'
import { buildAf, buildEqBands, EQ_FREQS, EQ_PRESETS } from './audioAf'

describe('buildEqBands', () => {
  it('flat → 全 0、長度 10', () => {
    expect(buildEqBands('flat')).toEqual([0,0,0,0,0,0,0,0,0,0])
  })
  it('未知 preset → flat', () => {
    expect(buildEqBands('nope')).toEqual([0,0,0,0,0,0,0,0,0,0])
  })
  it('每個 preset 長度皆 10、值在 ±12', () => {
    for (const p of Object.keys(EQ_PRESETS)) {
      const b = buildEqBands(p)
      expect(b.length).toBe(10)
      b.forEach((g) => { expect(g).toBeGreaterThanOrEqual(-12); expect(g).toBeLessThanOrEqual(12) })
    }
  })
})

describe('buildAf', () => {
  it('全關 → 空字串', () => {
    expect(buildAf({ eq: { enabled: false, bands: [3,0,0,0,0,0,0,0,0,0] }, normalize: false })).toBe('')
  })
  it('eq 開但全 0 → 空字串（無非零段不送 equalizer）', () => {
    expect(buildAf({ eq: { enabled: true, bands: [0,0,0,0,0,0,0,0,0,0] }, normalize: false })).toBe('')
  })
  it('eq 開、某段非 0 → 對應 equalizer 串', () => {
    const af = buildAf({ eq: { enabled: true, bands: [0,0,3,0,0,0,0,0,0,0] }, normalize: false })
    expect(af).toBe(`equalizer=f=${EQ_FREQS[2]}:t=o:w=1:g=3`)
  })
  it('多段 + 正規化 → 逗號串接、dynaudnorm 在尾', () => {
    const af = buildAf({ eq: { enabled: true, bands: [2,0,0,0,0,0,0,0,0,-4] }, normalize: true })
    expect(af).toBe(`equalizer=f=${EQ_FREQS[0]}:t=o:w=1:g=2,equalizer=f=${EQ_FREQS[9]}:t=o:w=1:g=-4,dynaudnorm`)
  })
  it('只正規化 → 只有 dynaudnorm', () => {
    expect(buildAf({ eq: { enabled: false, bands: [9,9,9,9,9,9,9,9,9,9] }, normalize: true })).toBe('dynaudnorm')
  })
})
