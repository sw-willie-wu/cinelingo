import { describe, it, expect } from 'vitest'
import { computeHasWhisperModel } from '../player/useModelDownloads'

describe('computeHasWhisperModel', () => {
  it('true iff a whisper model key is downloaded', () => {
    expect(computeHasWhisperModel(new Set())).toBe(false)
    expect(computeHasWhisperModel(new Set(['translate-4b']))).toBe(false) // 翻譯模型不算
    expect(computeHasWhisperModel(new Set(['turbo']))).toBe(true)
    expect(computeHasWhisperModel(new Set(['large-v3', 'translate-4b']))).toBe(true)
  })
})
