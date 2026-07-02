import { describe, it, expect } from 'vitest'
import { deriveIdle } from './usePlayer'

describe('deriveIdle', () => {
  it('should return true when path is null', () => {
    expect(deriveIdle(null)).toBe(true)
  })

  it('should return false when path is a string', () => {
    expect(deriveIdle('C:/a.mp4')).toBe(false)
  })

  it('should return false when path is a non-empty string', () => {
    expect(deriveIdle('/media/video.mkv')).toBe(false)
  })
})
