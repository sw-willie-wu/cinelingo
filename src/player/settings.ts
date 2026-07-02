import { LANGS } from './langs'
export type { SourceLang } from './langs'
export type ModelKey = 'small' | 'medium' | 'turbo' | 'large-v3'
export type TranslateModelKey = 'translate-4b' | 'translate-12b'
export type OutlineLevel = 'none' | 'thin' | 'mid' | 'thick'
export type SubBackground = 'none' | 'translucent'
export type VideoOutput = 'gpu' | 'gpu-next'
export type PersistedSource = { kind: 'system' } | { kind: 'process'; name: string } | { kind: 'inputDevice'; id: string }

export interface SubStyle {
  fontSize: number   // 1080p 參考高度下的 px 基準
  bottomPct: number  // 距底部 %
  color: string      // hex
  outline: OutlineLevel
  background: SubBackground
}
export interface ImageAdjust { brightness: number; contrast: number; saturation: number; gamma: number; hue: number; deband: boolean }
export interface EqState { enabled: boolean; preset: string; bands: number[] } // bands 長度 10
export interface Settings {
  version: number
  liveSubs: { enabled: boolean; model: ModelKey; sourceLang: string; saveSrt: boolean; overwriteOnParamChange: boolean; vad: { threshold: number; minSilenceMs: number }; display: { lines: number }; audioSource: PersistedSource | null; translateEnabled: boolean; translateTo: string; translateModel: TranslateModelKey }
  hardware: { accelEnabled: boolean | null }
  appearance: { maxWidthPct: number; primary: SubStyle; secondary: SubStyle }
  ui: { language: string }
  capture: { enabled: boolean; recordAudio: boolean }
  youtube: { quality: 'auto' | number }
  playback: { videoOutput: VideoOutput }
  floating: { x: number | null; y: number | null; width: number | null }
  video: ImageAdjust
  audio: { eq: EqState; normalize: boolean }
}

export const SETTINGS_VERSION = 1
const MODELS: ModelKey[] = ['small', 'medium', 'turbo', 'large-v3']
const TRANSLATE_MODELS: TranslateModelKey[] = ['translate-4b', 'translate-12b']
const LANG_VALUES = LANGS.map((l) => l.value)
const OUTLINES: OutlineLevel[] = ['none', 'thin', 'mid', 'thick']
const BGS: SubBackground[] = ['none', 'translucent']
export const YT_QUALITIES: readonly ('auto' | number)[] = ['auto', 2160, 1440, 1080, 720, 480, 360]
const VIDEO_OUTPUTS: VideoOutput[] = ['gpu', 'gpu-next']

export function defaultSettings(): Settings {
  return {
    version: SETTINGS_VERSION,
    liveSubs: { enabled: false, model: 'turbo', sourceLang: 'auto', saveSrt: true, overwriteOnParamChange: false, vad: { threshold: 0.5, minSilenceMs: 100 }, display: { lines: 3 }, audioSource: null, translateEnabled: false, translateTo: 'zh-Hant', translateModel: 'translate-4b' },
    hardware: { accelEnabled: null },
    appearance: {
      maxWidthPct: 80,
      primary: { fontSize: 28, bottomPct: 8, color: '#ffffff', outline: 'mid', background: 'none' },
      secondary: { fontSize: 18, bottomPct: 18, color: '#ffe14d', outline: 'thin', background: 'translucent' },
    },
    ui: { language: 'zh-TW' },
    capture: { enabled: false, recordAudio: false },
    youtube: { quality: 'auto' },
    playback: { videoOutput: 'gpu' },
    floating: { x: null, y: null, width: null },
    video: { brightness: 0, contrast: 0, saturation: 0, gamma: 0, hue: 0, deband: false },
    audio: { eq: { enabled: false, preset: 'flat', bands: [0,0,0,0,0,0,0,0,0,0] }, normalize: false },
  }
}

const pick = <T,>(v: unknown, allowed: readonly T[], def: T): T =>
  (allowed as readonly unknown[]).includes(v) ? (v as T) : def
const num = (v: unknown, def: number): number => (typeof v === 'number' && isFinite(v) ? v : def)
const numOrNull = (v: unknown): number | null => (typeof v === 'number' && isFinite(v) ? v : null)
const clamp = (v: number, lo: number, hi: number): number => (v < lo ? lo : v > hi ? hi : v)
const str = (v: unknown, def: string): string => (typeof v === 'string' ? v : def)
const bool = (v: unknown, def: boolean): boolean => (typeof v === 'boolean' ? v : def)
const obj = (v: unknown): Record<string, unknown> => (v && typeof v === 'object' ? (v as Record<string, unknown>) : {})
const persistedSource = (v: unknown): PersistedSource | null => {
  const o = obj(v)
  const kind = str(o.kind, '')
  if (kind === 'system') return { kind: 'system' }
  if (kind === 'process') {
    const name = str(o.name, '')
    if (name) return { kind: 'process', name }
  }
  if (kind === 'inputDevice') {
    const id = str(o.id, '')
    if (id) return { kind: 'inputDevice', id }
  }
  return null
}

function mergeStyle(raw: unknown, def: SubStyle): SubStyle {
  const r = obj(raw)
  return {
    fontSize: num(r.fontSize, def.fontSize),
    bottomPct: num(r.bottomPct, def.bottomPct),
    color: str(r.color, def.color),
    outline: pick(r.outline, OUTLINES, def.outline),
    background: pick(r.background, BGS, def.background),
  }
}

export function mergeSettings(raw: unknown): Settings {
  const d = defaultSettings()
  const o = obj(raw)
  const ls = obj(o.liveSubs), hw = obj(o.hardware), ap = obj(o.appearance), ui = obj(o.ui), cap = obj(o.capture), yt = obj(o.youtube)
  // Legacy migration: zh + chineseScript → zh-Hant / zh-Hans
  let sl = str(ls.sourceLang, d.liveSubs.sourceLang)
  if (sl === 'zh') sl = ls.chineseScript === 'simp' ? 'zh-Hans' : 'zh-Hant'
  const sourceLang = LANG_VALUES.includes(sl) ? sl : d.liveSubs.sourceLang
  return {
    version: SETTINGS_VERSION,
    liveSubs: {
      enabled: bool(ls.enabled, d.liveSubs.enabled),
      model: pick(ls.model, MODELS, d.liveSubs.model),
      sourceLang,
      saveSrt: bool(ls.saveSrt, d.liveSubs.saveSrt),
      overwriteOnParamChange: bool(ls.overwriteOnParamChange, d.liveSubs.overwriteOnParamChange),
      vad: (() => {
        const v = obj(ls.vad)
        return {
          threshold: clamp(Math.round(num(v.threshold, d.liveSubs.vad.threshold) * 100) / 100, 0, 1),
          minSilenceMs: clamp(Math.round(num(v.minSilenceMs, d.liveSubs.vad.minSilenceMs)), 0, 1000),
        }
      })(),
      display: (() => {
        const dp = obj(ls.display)
        return {
          lines: clamp(Math.round(num(dp.lines, d.liveSubs.display.lines)), 2, 5),
        }
      })(),
      audioSource: persistedSource(ls.audioSource),
      translateEnabled: bool(ls.translateEnabled, d.liveSubs.translateEnabled),
      // 目標語言須為明確語言（排除 'auto'）；非法 → 預設 zh-Hant
      translateTo: (() => {
        const v = str(ls.translateTo, d.liveSubs.translateTo)
        return v !== 'auto' && LANG_VALUES.includes(v) ? v : d.liveSubs.translateTo
      })(),
      translateModel: pick(ls.translateModel, TRANSLATE_MODELS, d.liveSubs.translateModel),
    },
    hardware: { accelEnabled: typeof hw.accelEnabled === 'boolean' ? hw.accelEnabled : null },
    appearance: {
      maxWidthPct: clamp(Math.round(num(ap.maxWidthPct, num(obj(ls.display).widthPct, d.appearance.maxWidthPct))), 50, 90),
      primary: mergeStyle(ap.primary, d.appearance.primary),
      secondary: mergeStyle(ap.secondary, d.appearance.secondary),
    },
    ui: { language: str(ui.language, d.ui.language) },
    capture: { enabled: bool(cap.enabled, d.capture.enabled), recordAudio: bool(cap.recordAudio, d.capture.recordAudio) },
    youtube: { quality: pick(yt.quality, YT_QUALITIES, d.youtube.quality) },
    playback: { videoOutput: pick(obj(o.playback).videoOutput, VIDEO_OUTPUTS, d.playback.videoOutput) },
    floating: (() => {
      const f = obj(o.floating)
      return { x: numOrNull(f.x), y: numOrNull(f.y), width: numOrNull(f.width) }
    })(),
    video: (() => {
      const v = obj(o.video), dv = d.video
      const ci = (x: unknown, def: number) => clamp(Math.round(num(x, def)), -100, 100)
      return {
        brightness: ci(v.brightness, dv.brightness), contrast: ci(v.contrast, dv.contrast),
        saturation: ci(v.saturation, dv.saturation), gamma: ci(v.gamma, dv.gamma), hue: ci(v.hue, dv.hue),
        deband: bool(v.deband, dv.deband),
      }
    })(),
    audio: (() => {
      const a = obj(o.audio), eq = obj(a.eq), da = d.audio
      const rawBands = Array.isArray(eq.bands) ? eq.bands : []
      const bands = da.eq.bands.map((def, i) => clamp(Math.round(num(rawBands[i], def)), -12, 12))
      return {
        eq: { enabled: bool(eq.enabled, da.eq.enabled), preset: str(eq.preset, da.eq.preset), bands },
        normalize: bool(a.normalize, da.normalize),
      }
    })(),
  }
}

export function outlineToTextShadow(level: OutlineLevel): string {
  if (level === 'none') return 'none'
  const w = level === 'thin' ? 1 : level === 'mid' ? 2 : 3
  const parts: string[] = []
  for (let x = -w; x <= w; x++) {
    for (let y = -w; y <= w; y++) {
      if (x !== 0 || y !== 0) parts.push(`${x}px ${y}px 0 #000`)
    }
  }
  if (level === 'thick') parts.push('0 0 6px #000')
  return parts.join(', ')
}
export function scaleFontPx(baseAt1080: number, containerH: number): number {
  return Math.max(12, Math.min(80, baseAt1080 * (containerH / 1080)))
}
export function subTextStyle(style: SubStyle, containerH: number): Record<string, string> {
  const css: Record<string, string> = {
    fontSize: `${scaleFontPx(style.fontSize, containerH)}px`,
    color: style.color,
    textShadow: outlineToTextShadow(style.outline),
    // padding/borderRadius 永遠存在 → 開關底色時盒子大小不變、文字不飄移
    padding: '0.12em 0.5em',
    borderRadius: '5px',
  }
  if (style.background === 'translucent') {
    css.background = 'rgba(0,0,0,0.55)'
  }
  return css
}
