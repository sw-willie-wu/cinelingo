import { it, expect } from 'vitest'
import { pickCueText } from '../player/subtitles'
const cue = { id: '1', sessionId: '', startSec: 1, endSec: 2, sourceText: 'hello', lang: null, status: 'final' as const, translations: { 'zh-Hant': '你好' } }

it('pickCueText: off → sourceText', () => {
  expect(pickCueText(cue, 'off')).toBe('hello')
})
it('pickCueText: target present → translation', () => {
  expect(pickCueText(cue, 'zh-Hant')).toBe('你好')
})
it('pickCueText: target missing → fallback sourceText', () => {
  expect(pickCueText(cue, 'ja')).toBe('hello')
  expect(pickCueText({ ...cue, translations: undefined }, 'zh-Hant')).toBe('hello')
})
