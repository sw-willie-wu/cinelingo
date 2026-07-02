import { describe, it, expect } from 'vitest'
import { normKey, trackToStored, restoreTrackSource, coerceStoredEntry, type SubFile, type StoredTrack } from './subMemory'

const f = (id: string, path: string | null): SubFile => ({ id, path })

describe('normKey', () => {
  it('lowercases and unifies separators (≡ rust normalize)', () => {
    expect(normKey('C:\\Movies\\Film.MKV')).toBe('c:/movies/film.mkv')
    expect(normKey('C:/Movies/Film.MKV')).toBe(normKey('c:\\movies\\film.mkv'))
  })
})

describe('trackToStored', () => {
  const files = [f('f1', 'C:/a.srt')]
  it('off/live pass through', () => {
    expect(trackToStored({ source: 'off', delaySec: 0, translateTo: 'off' }, files)).toEqual({ source: 'off', delaySec: 0, translateTo: 'off' })
    expect(trackToStored({ source: 'live', delaySec: 1, translateTo: 'zh-Hant' }, files)).toEqual({ source: 'live', delaySec: 1, translateTo: 'zh-Hant' })
  })
  it('fileId → its absolute path', () => {
    expect(trackToStored({ source: 'f1', delaySec: 0.5, translateTo: 'off' }, files)).toEqual({ source: 'C:/a.srt', delaySec: 0.5, translateTo: 'off' })
  })
})

describe('restoreTrackSource', () => {
  const files = [f('f1', 'C:/a.srt')]
  it('off → off', () => { expect(restoreTrackSource('off', files, true)).toBe('off') })
  it('live gated by master', () => {
    expect(restoreTrackSource('live', files, true)).toBe('live')
    expect(restoreTrackSource('live', files, false)).toBe('off')
  })
  it('path → matching fileId', () => { expect(restoreTrackSource('C:/a.srt', files, true)).toBe('f1') })
  it('missing path → off', () => { expect(restoreTrackSource('C:/gone.srt', files, true)).toBe('off') })
})

describe('coerceStoredEntry', () => {
  const off: StoredTrack = { source: 'off', delaySec: 0, translateTo: 'off' }
  it('fully-formed entry passes through unchanged', () => {
    const entry = { manualFiles: ['a.srt', 'b.srt'], primary: { source: 'live', delaySec: 0.5, translateTo: 'zh-Hant' }, secondary: { source: 'off', delaySec: 0, translateTo: 'off' } }
    expect(coerceStoredEntry(entry)).toEqual(entry)
  })
  it('missing primary → defaults to {source:off, delaySec:0, translateTo:off}', () => {
    const result = coerceStoredEntry({ manualFiles: [], secondary: { source: 'C:/sub.srt', delaySec: 1 } })
    expect(result.primary).toEqual(off)
    expect(result.secondary).toEqual({ source: 'C:/sub.srt', delaySec: 1, translateTo: 'off' })
  })
  it('missing secondary → defaults to {source:off, delaySec:0, translateTo:off}', () => {
    const result = coerceStoredEntry({ manualFiles: [], primary: { source: 'live', delaySec: 2 } })
    expect(result.secondary).toEqual(off)
    expect(result.primary).toEqual({ source: 'live', delaySec: 2, translateTo: 'off' })
  })
  it('missing/garbage manualFiles → []', () => {
    expect(coerceStoredEntry({ primary: off, secondary: off }).manualFiles).toEqual([])
    expect(coerceStoredEntry({ manualFiles: null, primary: off, secondary: off }).manualFiles).toEqual([])
    expect(coerceStoredEntry({ manualFiles: [42, 'valid.srt', true], primary: off, secondary: off }).manualFiles).toEqual(['valid.srt'])
  })
  it('entirely empty object → all defaults', () => {
    expect(coerceStoredEntry({})).toEqual({ manualFiles: [], primary: off, secondary: off })
  })
})
