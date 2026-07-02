import { it, expect } from 'vitest'
import { mergeCachedTranslations } from '../player/subtitles'
const mk = (id: string, s: number, e: number) => ({ id, sessionId: '', startSec: s, endSec: e, sourceText: id, lang: null, status: 'final' as const })

it('mergeCachedTranslations: 以 round(startSec*1000) 精確配對、不誤綁重疊 cue', () => {
  const cues = [mk('a', 1.0, 5.0), mk('b', 2.0, 3.0)] // a,b 時間重疊
  const cached = [
    { id: '1000', translations: { 'zh-Hant': 'A譯' } },
    { id: '2000', translations: { 'zh-Hant': 'B譯' } },
  ]
  const got = mergeCachedTranslations(cues, cached)
  expect(got[0].translations?.['zh-Hant']).toBe('A譯') // 1.0s → 1000ms
  expect(got[1].translations?.['zh-Hant']).toBe('B譯') // 2.0s → 2000ms（未誤綁到 a）
})
it('mergeCachedTranslations: 無對應 → 保留原 cue', () => {
  const got = mergeCachedTranslations([mk('a', 9.0, 10.0)], [])
  expect(got[0].translations).toBeUndefined()
})
