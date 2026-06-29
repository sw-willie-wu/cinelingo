import { describe, it, expect } from 'vitest'
import { liveBlocks, type Cue } from '../player/subtitles'
const f = (id: string, src: string, tgt?: string): Cue => ({
  id, sessionId: 's', startSec: 0, endSec: 1, sourceText: src, lang: null, status: 'final', targetText: tgt,
})
describe('liveBlocks', () => {
  it('returns last n final cues as blocks with target', () => {
    const cues = [f('1','A。'), f('2','B。','譯B'), f('3','C。')]
    const b = liveBlocks(cues, null, 2)
    expect(b.map(x => x.id)).toEqual(['2','3'])
    expect(b[0].target).toBe('譯B')
    expect(b[1].target).toBeUndefined()
    expect(b[0].sourceLines).toEqual(['B。'])
  })
  it('appends interim block (no target) at end', () => {
    const it_ = { id: 's:interim', sessionId:'s', startSec:0, endSec:1, sourceText:'half', lang:null, status:'interim' as const }
    const b = liveBlocks([f('1','A。')], it_, 3, 16)
    expect(b[b.length-1].interim).toBe(true)
    expect(b[b.length-1].target).toBeUndefined()
  })
  it('empty when no finals and no interim', () => {
    expect(liveBlocks([], null, 3)).toEqual([])
  })
})
