import { describe, it, expect } from 'vitest'
import { defaultSettings, mergeSettings, SETTINGS_VERSION } from '../player/settings'
import { outlineToTextShadow, scaleFontPx, subTextStyle } from '../player/settings'

describe('settings defaults + merge', () => {
  it('defaults have expected shape', () => {
    const d = defaultSettings()
    expect(d.version).toBe(SETTINGS_VERSION)
    expect(d.liveSubs.model).toBe('turbo')
    expect(d.liveSubs.saveSrt).toBe(true)
    expect(d.hardware.accelEnabled).toBeNull()
    expect(d.appearance.primary.color).toBe('#ffffff')
  })
  it('merge of undefined/garbage → defaults', () => {
    expect(mergeSettings(undefined)).toEqual(defaultSettings())
    expect(mergeSettings(42)).toEqual(defaultSettings())
    expect(mergeSettings({})).toEqual(defaultSettings())
  })
  it('valid overrides kept, missing filled', () => {
    const m = mergeSettings({ liveSubs: { model: 'small' } })
    expect(m.liveSubs.model).toBe('small')
    expect(m.liveSubs.sourceLang).toBe('auto')
  })
  it('overwriteOnParamChange: default false, true kept, non-bool → false', () => {
    expect(defaultSettings().liveSubs.overwriteOnParamChange).toBe(false)
    expect(mergeSettings({ liveSubs: { model: 'turbo' } }).liveSubs.overwriteOnParamChange).toBe(false)
    expect(mergeSettings({ liveSubs: { overwriteOnParamChange: true } }).liveSubs.overwriteOnParamChange).toBe(true)
    expect(mergeSettings({ liveSubs: { overwriteOnParamChange: 'x' } }).liveSubs.overwriteOnParamChange).toBe(false)
  })
  it('invalid enum falls back to default', () => {
    expect(mergeSettings({ liveSubs: { model: 'bogus' } }).liveSubs.model).toBe('turbo')
    expect(mergeSettings({ appearance: { primary: { outline: 'x' } } }).appearance.primary.outline).toBe('mid')
  })
  it('accelEnabled: explicit false kept, missing → null', () => {
    expect(mergeSettings({ hardware: { accelEnabled: false } }).hardware.accelEnabled).toBe(false)
    expect(mergeSettings({ hardware: {} }).hardware.accelEnabled).toBeNull()
  })
  it('version normalised to current', () => {
    expect(mergeSettings({ version: 0 }).version).toBe(SETTINGS_VERSION)
  })
  it('migrates legacy zh+trad → zh-Hant', () => {
    expect(mergeSettings({ liveSubs: { sourceLang: 'zh', chineseScript: 'trad' } } as any).liveSubs.sourceLang).toBe('zh-Hant')
  })
  it('migrates legacy zh+simp → zh-Hans', () => {
    expect(mergeSettings({ liveSubs: { sourceLang: 'zh', chineseScript: 'simp' } } as any).liveSubs.sourceLang).toBe('zh-Hans')
  })
  it('bare zh → zh-Hant', () => {
    expect(mergeSettings({ liveSubs: { sourceLang: 'zh' } } as any).liveSubs.sourceLang).toBe('zh-Hant')
  })
  it('keeps valid lang ja', () => {
    expect(mergeSettings({ liveSubs: { sourceLang: 'ja' } } as any).liveSubs.sourceLang).toBe('ja')
  })
  it('vad defaults', () => {
    const d = defaultSettings()
    expect(d.liveSubs.vad).toEqual({ threshold: 0.5, minSilenceMs: 100 })
  })
  it('missing vad → defaults', () => {
    expect(mergeSettings({ liveSubs: {} }).liveSubs.vad).toEqual({ threshold: 0.5, minSilenceMs: 100 })
  })
  it('missing single vad sub-field → that field default', () => {
    expect(mergeSettings({ liveSubs: { vad: { threshold: 0.8 } } } as any).liveSubs.vad)
      .toEqual({ threshold: 0.8, minSilenceMs: 100 })
  })
  it('vad threshold clamped to 0..1', () => {
    expect(mergeSettings({ liveSubs: { vad: { threshold: 5 } } } as any).liveSubs.vad.threshold).toBe(1)
    expect(mergeSettings({ liveSubs: { vad: { threshold: -3 } } } as any).liveSubs.vad.threshold).toBe(0)
  })
  it('vad minSilenceMs clamped to 0..1000 and rounded', () => {
    expect(mergeSettings({ liveSubs: { vad: { minSilenceMs: 99999 } } } as any).liveSubs.vad.minSilenceMs).toBe(1000)
    expect(mergeSettings({ liveSubs: { vad: { minSilenceMs: -50 } } } as any).liveSubs.vad.minSilenceMs).toBe(0)
    expect(mergeSettings({ liveSubs: { vad: { minSilenceMs: 123.7 } } } as any).liveSubs.vad.minSilenceMs).toBe(124)
  })
  it('vad threshold rounded to 2dp (mirrors argv precision)', () => {
    expect(mergeSettings({ liveSubs: { vad: { threshold: 0.5004 } } } as any).liveSubs.vad.threshold).toBe(0.5)
    expect(mergeSettings({ liveSubs: { vad: { threshold: 0.337 } } } as any).liveSubs.vad.threshold).toBe(0.34)
  })
  it('vad garbage (NaN/string) → defaults', () => {
    expect(mergeSettings({ liveSubs: { vad: { threshold: 'x', minSilenceMs: NaN } } } as any).liveSubs.vad)
      .toEqual({ threshold: 0.5, minSilenceMs: 100 })
  })
  it('capture defaults + merge', () => {
    expect(defaultSettings().capture).toEqual({ enabled: false })
    expect(mergeSettings({}).capture).toEqual({ enabled: false })
    expect(mergeSettings({ capture: { enabled: true } }).capture.enabled).toBe(true)
    expect(mergeSettings({ capture: { enabled: 'x' } } as any).capture.enabled).toBe(false)
  })
  it('maxWidthPct: default 80, lives on appearance', () => {
    expect(defaultSettings().appearance.maxWidthPct).toBe(80)
    expect(mergeSettings({}).appearance.maxWidthPct).toBe(80)
  })
  it('maxWidthPct migrates from legacy liveSubs.display.widthPct', () => {
    const m = mergeSettings({ liveSubs: { display: { widthPct: 65, lines: 4 } } } as any)
    expect(m.appearance.maxWidthPct).toBe(65)   // 舊值遷移保留
    expect(m.liveSubs.display.lines).toBe(4)     // 行數原地不動
  })
  it('maxWidthPct: new appearance field wins over legacy widthPct', () => {
    const m = mergeSettings({ appearance: { maxWidthPct: 55 }, liveSubs: { display: { widthPct: 80 } } } as any)
    expect(m.appearance.maxWidthPct).toBe(55)
  })
  it('maxWidthPct clamped to 50..90', () => {
    expect(mergeSettings({ appearance: { maxWidthPct: 200 } } as any).appearance.maxWidthPct).toBe(90)
    expect(mergeSettings({ appearance: { maxWidthPct: 10 } } as any).appearance.maxWidthPct).toBe(50)
  })
  it('youtube.quality 缺 → 預設 auto', () => {
    expect(mergeSettings({}).youtube.quality).toBe('auto')
  })
  it('youtube.quality 有效值保留', () => {
    expect(mergeSettings({ youtube: { quality: 2160 } }).youtube.quality).toBe(2160)
  })
  it("youtube.quality 'auto' 仍為有效值（供畫質選單手動選）", () => {
    expect(mergeSettings({ youtube: { quality: 'auto' } }).youtube.quality).toBe('auto')
  })
  it('youtube.quality 非法值 → 預設 auto', () => {
    expect(mergeSettings({ youtube: { quality: 999 } }).youtube.quality).toBe('auto')
  })
  it('video/audio 預設值', () => {
    const d = defaultSettings()
    expect(d.video).toEqual({ brightness: 0, contrast: 0, saturation: 0, gamma: 0, hue: 0, deband: false })
    expect(d.audio.normalize).toBe(false)
    expect(d.audio.eq).toEqual({ enabled: false, preset: 'flat', bands: [0,0,0,0,0,0,0,0,0,0] })
    expect('secondaryEnabled' in d.appearance).toBe(false)
  })
  it('mergeSettings 回填 video/audio、clamp 越界、丟棄 secondaryEnabled', () => {
    const m = mergeSettings({
      appearance: { secondaryEnabled: true },
      video: { brightness: 999, contrast: -999, deband: true },
      audio: { eq: { enabled: true, preset: 'rock', bands: [99,-99,0,0,0,0,0,0,0,0] }, normalize: true },
    })
    expect('secondaryEnabled' in m.appearance).toBe(false)
    expect(m.video.brightness).toBe(100)
    expect(m.video.contrast).toBe(-100)
    expect(m.video.deband).toBe(true)
    expect(m.audio.eq.enabled).toBe(true)
    expect(m.audio.eq.bands[0]).toBe(12)
    expect(m.audio.eq.bands[1]).toBe(-12)
    expect(m.audio.normalize).toBe(true)
  })
  it('mergeSettings 缺 video/audio → 補預設', () => {
    const m = mergeSettings({})
    expect(m.video.saturation).toBe(0)
    expect(m.audio.eq.bands.length).toBe(10)
  })
  it('audioSource defaults null on legacy', () => {
    const merged = mergeSettings({ liveSubs: { model: 'turbo' } } as any)
    expect(merged.liveSubs.audioSource).toBeNull()
  })
})

describe('playback.videoOutput', () => {
  it('預設 gpu', () => {
    expect(defaultSettings().playback.videoOutput).toBe('gpu')
    expect(mergeSettings({}).playback.videoOutput).toBe('gpu')
  })
  it('gpu-next 有效值保留', () => {
    expect(mergeSettings({ playback: { videoOutput: 'gpu-next' } }).playback.videoOutput).toBe('gpu-next')
  })
  it('非法值 → 預設 gpu', () => {
    expect(mergeSettings({ playback: { videoOutput: 'vulkan' } }).playback.videoOutput).toBe('gpu')
  })
})

describe('floating settings', () => {
  it('defaults to all-null', () => {
    expect(defaultSettings().floating).toEqual({ x: null, y: null, width: null })
  })
  it('round-trips numbers', () => {
    const s = mergeSettings({ floating: { x: 100, y: 200, width: 800 } })
    expect(s.floating).toEqual({ x: 100, y: 200, width: 800 })
  })
  it('coerces non-numbers to null (not default-number)', () => {
    const s = mergeSettings({ floating: { x: 'oops', y: null, width: 800 } })
    expect(s.floating).toEqual({ x: null, y: null, width: 800 })
  })
  it('missing floating → all-null (backward compat)', () => {
    const s = mergeSettings({ version: 1 })
    expect(s.floating).toEqual({ x: null, y: null, width: null })
  })
  it('rejects NaN/Infinity → null (exercises isFinite guard)', () => {
    const s = mergeSettings({ floating: { x: NaN, y: Infinity, width: 800 } } as any)
    expect(s.floating).toEqual({ x: null, y: null, width: 800 })
  })
})

describe('appearance → css', () => {
  it('outline none → none', () => { expect(outlineToTextShadow('none')).toBe('none') })
  it('outline thick has multiple shadows', () => { expect(outlineToTextShadow('thick').split(',').length).toBeGreaterThan(2) })
  it('scaleFontPx scales by container height vs 1080 and clamps', () => {
    expect(scaleFontPx(28, 1080)).toBeCloseTo(28)
    expect(scaleFontPx(28, 540)).toBeCloseTo(14)
    expect(scaleFontPx(28, 100)).toBe(12)     // clamp min
    expect(scaleFontPx(60, 4320)).toBe(80)    // clamp max
  })
  it('subTextStyle composes; translucent adds background+padding', () => {
    const none = subTextStyle({ fontSize: 28, bottomPct: 8, color: '#fff', outline: 'mid', background: 'none' }, 1080)
    expect(none.color).toBe('#fff')
    expect(none.background).toBeUndefined()
    const bg = subTextStyle({ fontSize: 28, bottomPct: 8, color: '#fff', outline: 'mid', background: 'translucent' }, 1080)
    expect(bg.background).toContain('rgba')
    expect(bg.padding).toBeDefined()
  })
})
