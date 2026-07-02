import { it, expect } from 'vitest'
import { selectUpcomingUntranslated } from '../player/subtitles'
const mk = (id: string, s: number, e: number, tr?: Record<string,string>) =>
  ({ id, sessionId: '', startSec: s, endSec: e, sourceText: id, lang: null, status: 'final' as const, translations: tr })

it('selectUpcomingUntranslated: 前方窗內、缺該 target 者', () => {
  const cues = [
    mk('past', 0, 1),                         // 已過
    mk('cur', 9, 11),                          // 涵蓋 playhead=10
    mk('soon', 12, 13),                        // 窗內
    mk('done', 14, 15, { 'zh-Hant': '有' }),   // 窗內但已翻 → 排除
    mk('far', 30, 31),                         // 窗外
  ]
  const got = selectUpcomingUntranslated(cues, 10, 10, 'zh-Hant').map(c => c.id)
  expect(got).toEqual(['cur', 'soon'])
})
