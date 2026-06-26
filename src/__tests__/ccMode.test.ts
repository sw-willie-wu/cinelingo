import { describe, it, expect } from 'vitest'
import { ccMode } from '../player/useSubtitles'

describe('ccMode', () => {
  it('playing → file', () => {
    expect(ccMode(true, false)).toBe('file')
  })
  it('idle + armed → external', () => {
    expect(ccMode(false, true)).toBe('external')
  })
  it('idle + not armed → disabled', () => {
    expect(ccMode(false, false)).toBe('disabled')
  })
})
