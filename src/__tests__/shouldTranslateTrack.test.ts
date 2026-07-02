import { it, expect } from 'vitest'
import { shouldTranslateTrack } from '../player/useSubtitles'
const T = (source: string, translateTo: string) => ({ source, translateTo })
it('shouldTranslateTrack: 三重條件 + translateTo', () => {
  expect(shouldTranslateTrack(T('live', 'zh-Hant'), false, true, true)).toBe(true)
  expect(shouldTranslateTrack(T('f1', 'zh-Hant'), false, true, true)).toBe(true) // 字幕檔軌
  expect(shouldTranslateTrack(T('live', 'off'), false, true, true)).toBe(false)   // off
  expect(shouldTranslateTrack(T('live', 'zh-Hant'), true, true, true)).toBe(false)  // loopback
  expect(shouldTranslateTrack(T('live', 'zh-Hant'), false, false, true)).toBe(false) // 無來源(arm 空隙)
  expect(shouldTranslateTrack(T('live', 'zh-Hant'), false, true, false)).toBe(false) // 引擎未 ready
})
