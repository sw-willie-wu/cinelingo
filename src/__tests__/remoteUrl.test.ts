import { describe, it, expect } from 'vitest'
import { isHttpUrl, canonicalRemoteId, playlistContext } from '../player/remoteUrl'

describe('isHttpUrl', () => {
  it('matches http(s) URLs (youtube + general sites — all tried via yt-dlp)', () => {
    expect(isHttpUrl('https://www.youtube.com/watch?v=abc123')).toBe(true)
    expect(isHttpUrl('http://youtube.com/watch?v=abc123&t=5s')).toBe(true)
    expect(isHttpUrl('https://youtu.be/abc123')).toBe(true)
    expect(isHttpUrl('https://example.com/video.mp4')).toBe(true)
    expect(isHttpUrl('https://vimeo.com/12345')).toBe(true)
  })
  it('trims surrounding whitespace', () => {
    expect(isHttpUrl('  https://youtube.com/watch?v=abc  ')).toBe(true)
  })
  it('rejects non-http(s) schemes / non-urls / plain text', () => {
    expect(isHttpUrl('hello world')).toBe(false)
    expect(isHttpUrl('')).toBe(false)
    expect(isHttpUrl('not a url youtube.com')).toBe(false)
    expect(isHttpUrl('ftp://example.com/x')).toBe(false)
    expect(isHttpUrl('file:///c:/x.mp4')).toBe(false)
  })
})

describe('canonicalRemoteId', () => {
  const CANON = 'https://www.youtube.com/watch?v=ABC123defGH'
  it('canonicalizes all YouTube URL forms to watch?v=<id>', () => {
    expect(canonicalRemoteId('https://www.youtube.com/watch?v=ABC123defGH')).toBe(CANON)
    expect(canonicalRemoteId('https://youtu.be/ABC123defGH')).toBe(CANON)
    expect(canonicalRemoteId('https://m.youtube.com/watch?v=ABC123defGH&t=30s')).toBe(CANON)
    expect(canonicalRemoteId('https://www.youtube.com/watch?v=ABC123defGH&list=PLxxx')).toBe(CANON)
    expect(canonicalRemoteId('https://music.youtube.com/watch?v=ABC123defGH')).toBe(CANON)
    expect(canonicalRemoteId('https://www.youtube.com/live/ABC123defGH')).toBe(CANON)
    expect(canonicalRemoteId('https://www.youtube.com/shorts/ABC123defGH')).toBe(CANON)
    expect(canonicalRemoteId('https://www.youtube.com/embed/ABC123defGH')).toBe(CANON)
    expect(canonicalRemoteId('https://www.youtube.com/v/ABC123defGH')).toBe(CANON)
    expect(canonicalRemoteId('https://youtu.be/ABC123defGH?t=30')).toBe(CANON)
    expect(canonicalRemoteId('  https://youtu.be/ABC123defGH  ')).toBe(CANON) // trim
  })
  it('passes through non-YouTube / lookalike / bad input unchanged', () => {
    expect(canonicalRemoteId('https://vimeo.com/12345')).toBe('https://vimeo.com/12345')
    expect(canonicalRemoteId('https://example.com/a')).toBe('https://example.com/a')
    // lookalike host 不可被誤正規化（守 exact-host 比對）
    expect(canonicalRemoteId('https://evil-youtube.com/watch?v=ABC123defGH')).toBe('https://evil-youtube.com/watch?v=ABC123defGH')
    expect(canonicalRemoteId('not a url')).toBe('not a url')
    expect(canonicalRemoteId('')).toBe('')
  })
})

describe('playlistContext', () => {
  it('watch?v=ID&list= → 帶 videoId', () => {
    expect(playlistContext('https://www.youtube.com/watch?v=abcdefghijk&list=PL123'))
      .toEqual({ listId: 'PL123', videoId: 'abcdefghijk' })
  })
  it('純 playlist?list= → videoId null', () => {
    expect(playlistContext('https://www.youtube.com/playlist?list=PL123'))
      .toEqual({ listId: 'PL123', videoId: null })
  })
  it('無 list 參數 → null', () => {
    expect(playlistContext('https://www.youtube.com/watch?v=abcdefghijk')).toBeNull()
  })
  it('非 YouTube host → null（免 evil-youtube.com 誤中）', () => {
    expect(playlistContext('https://evil-youtube.com/watch?v=x&list=PL1')).toBeNull()
  })
  it('壞 URL → null', () => {
    expect(playlistContext('not a url')).toBeNull()
  })
})
