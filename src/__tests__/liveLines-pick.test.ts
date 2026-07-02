import { describe, it, expect } from 'vitest'
import { liveLines, type Cue } from '../player/subtitles'
const f = (id: string, src: string, tr?: Record<string, string>): Cue => ({
  id, sessionId: 's', startSec: 0, endSec: 1, sourceText: src, lang: null, status: 'final', translations: tr,
})
describe('liveLines field picker', () => {
  it('defaults to sourceText', () => {
    const r = liveLines([f('1', 'あ。'), f('2', 'い。')], null, 3)
    expect(r.lines).toEqual(['あ。', 'い。'])
  })
  it('picks translations[lang] when a picker is given', () => {
    const pick = (c: Cue) => c.translations?.['zh-Hant'] ?? c.sourceText
    const r = liveLines([f('1', 'あ。', { 'zh-Hant': '啊。' }), f('2', 'い。')], null, 3, undefined, pick)
    expect(r.lines).toEqual(['啊。', 'い。']) // 2nd falls back to source (no translation yet)
  })
  it('interim always shows source text (emitted untranslated)', () => {
    const interim: Cue = { id: 's:i', sessionId: 's', startSec: 0, endSec: 1, sourceText: 'half', lang: null, status: 'interim' }
    const pick = (c: Cue) => c.translations?.['zh-Hant'] ?? c.sourceText
    const r = liveLines([], interim, 3, 16, pick)
    expect(r.interimLines).toEqual(['half'])
  })
})
