import { describe, it, expect } from 'vitest'
import { provisionSummary } from './provision'

describe('provisionSummary', () => {
  it('空 missing → total 0', () => {
    expect(provisionSummary({ backendKind: 'cuda', missing: [] })).toEqual({
      missing: [], totalMb: 0,
    })
  })
  it('加總大小', () => {
    const s = provisionSummary({
      backendKind: 'cuda',
      missing: [
        { kind: 'model', sizeMb: 1549 },
        { kind: 'vad', sizeMb: 1 },
        { kind: 'backend', sizeMb: 439 },
      ],
    })
    expect(s.totalMb).toBe(1989)
  })
  it('有缺項 → 加總大小', () => {
    const s = provisionSummary({ backendKind: 'cpu', missing: [{ kind: 'ffmpeg', sizeMb: 80 }] })
    expect(s.totalMb).toBe(80)
  })
})
