import { describe, it, expect } from 'vitest'
import { pickVideoFormat, pickForPref, autoCapHeight } from '../player/quality'
import type { VideoFormat } from '../player/backend'

const V = (height: number): VideoFormat => ({ itag: String(height), height, fps: 30, codec: 'vp9', tbr: height, url: `u${height}` })
const vids: VideoFormat[] = [V(2160), V(1080), V(720), V(480)] // 已降冪

describe('pickVideoFormat', () => {
  it('auto → 最高', () => { expect(pickVideoFormat(vids, 'auto')?.height).toBe(2160) })
  it('命中', () => { expect(pickVideoFormat(vids, 1080)?.height).toBe(1080) })
  it('沒命中 → 取 ≤ 該值的最高', () => { expect(pickVideoFormat(vids, 1440)?.height).toBe(1080) })
  it('全高於 → 取最低可用', () => { expect(pickVideoFormat([V(2160), V(1440)], 720)?.height).toBe(1440) })
  it('空清單 → null', () => { expect(pickVideoFormat([], 'auto')).toBeNull() })
})

describe('pickForPref（auto 套螢幕上限）', () => {
  it('數值 → 等同 pickVideoFormat（可超螢幕、手動 2K/4K）', () => {
    expect(pickForPref(vids, 2160)?.height).toBe(2160)
    expect(pickForPref(vids, 1080)?.height).toBe(1080)
  })
  it("'auto' → 不超過螢幕實體上限的最高", () => {
    const cap = autoCapHeight()
    const v = pickForPref(vids, 'auto')
    expect(v).not.toBeNull()
    if (vids.some((x) => x.height <= cap)) expect(v!.height).toBeLessThanOrEqual(cap)
  })
})
