import { describe, it, expect } from 'vitest'
import { liveBlockedByNoModel } from '../player/useSubtitles'
describe('liveBlockedByNoModel', () => {
  it('blocks live start only when needed and no model', () => {
    expect(liveBlockedByNoModel(true, false)).toBe(true)
    expect(liveBlockedByNoModel(true, true)).toBe(false)
    expect(liveBlockedByNoModel(false, false)).toBe(false)
  })
})
