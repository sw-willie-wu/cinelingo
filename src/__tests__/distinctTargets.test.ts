import { describe, it, expect } from 'vitest'
import { distinctTargets } from '../player/subtitles'
const live = (to: string) => ({ source: 'live', translateTo: to })
const off = () => ({ source: 'off', translateTo: 'off' })
describe('distinctTargets', () => {
  it('empty when master disabled', () => {
    expect(distinctTargets(live('zh-Hant'), live('zh-Hans'), false)).toEqual([])
  })
  it('collects distinct non-off targets from live tracks', () => {
    expect(distinctTargets(live('zh-Hant'), live('zh-Hans'), true)).toEqual(['zh-Hant', 'zh-Hans'])
  })
  it('dedupes identical targets', () => {
    expect(distinctTargets(live('zh-Hant'), live('zh-Hant'), true)).toEqual(['zh-Hant'])
  })
  it('ignores off tracks and non-live sources', () => {
    expect(distinctTargets(off(), live('zh-Hant'), true)).toEqual(['zh-Hant'])
    expect(distinctTargets({ source: 'file-1', translateTo: 'zh-Hant' }, off(), true)).toEqual([])
  })
})
