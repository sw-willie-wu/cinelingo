import { describe, it, expect } from 'vitest'
import { basename } from '../player/path'

describe('basename', () => {
  it('Windows 反斜線', () => {
    expect(basename('C:\\a\\b\\movie.mkv')).toBe('movie.mkv')
  })
  it('正斜線', () => {
    expect(basename('/a/b/clip.mp4')).toBe('clip.mp4')
  })
  it('無分隔', () => {
    expect(basename('x.mkv')).toBe('x.mkv')
  })
})
