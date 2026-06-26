import { describe, it, expect } from 'vitest'
import { provisionSummary } from './provision'

describe('provisionSummary', () => {
  it('空 missing → total 0、noModel false', () => {
    expect(provisionSummary({ backendKind: 'cuda', missing: [] })).toEqual({
      missing: [], totalMb: 0, noModel: false,
    })
  })
  it('加總大小並偵測有模型缺項', () => {
    const s = provisionSummary({
      backendKind: 'cuda',
      missing: [
        { kind: 'model', sizeMb: 1549 },
        { kind: 'vad', sizeMb: 1 },
        { kind: 'backend', sizeMb: 439 },
      ],
    })
    expect(s.totalMb).toBe(1989)
    expect(s.noModel).toBe(true)
  })
  it('無 model 缺項 → noModel false', () => {
    const s = provisionSummary({ backendKind: 'cpu', missing: [{ kind: 'ffmpeg', sizeMb: 80 }] })
    expect(s.noModel).toBe(false)
    expect(s.totalMb).toBe(80)
  })
})
