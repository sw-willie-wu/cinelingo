import { describe, it, expect } from 'vitest'
import { coercePlaybackEntry, mergeEntry, isDefaultEntry } from './playbackMemory'

describe('coercePlaybackEntry', () => {
  it('壞格式 → 空 entry', () => {
    expect(coercePlaybackEntry(null)).toEqual({})
    expect(coercePlaybackEntry({ speed: 'x', audioDelaySec: {} })).toEqual({})
  })
  it('合法值保留', () => {
    expect(coercePlaybackEntry({ speed: 1.5, audioDelaySec: -0.2 })).toEqual({ speed: 1.5, audioDelaySec: -0.2 })
  })
})
describe('isDefaultEntry', () => {
  it('speed 1 且 delay 0（或缺）→ 視為預設（可刪 key）', () => {
    expect(isDefaultEntry({})).toBe(true)
    expect(isDefaultEntry({ speed: 1, audioDelaySec: 0 })).toBe(true)
    expect(isDefaultEntry({ speed: 1.25 })).toBe(false)
    expect(isDefaultEntry({ audioDelaySec: 0.1 })).toBe(false)
  })
})
describe('mergeEntry', () => {
  it('部分更新合併', () => {
    expect(mergeEntry({ speed: 1.5 }, { audioDelaySec: 0.1 })).toEqual({ speed: 1.5, audioDelaySec: 0.1 })
  })
})
