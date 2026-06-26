import { describe, it, expect, vi } from 'vitest'
import { resolveSource, normalizeLevel } from './useAudioSource'
import type { AudioSources } from './backend'

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => {}) }))

const live: AudioSources = { processes: [{ pid: 9, name: 'chrome.exe' }], inputDevices: [] }

describe('resolveSource', () => {
  it('resolves process by name → pid', () => {
    expect(resolveSource({ kind: 'process', name: 'chrome.exe' }, live)).toEqual({ kind: 'process', pid: 9 })
  })
  it('falls back to system when name gone', () => {
    expect(resolveSource({ kind: 'process', name: 'gone.exe' }, live)).toEqual({ kind: 'system' })
  })
  it('input device passes id through', () => {
    expect(resolveSource({ kind: 'inputDevice', id: 'd1' }, live)).toEqual({ kind: 'inputDevice', id: 'd1' })
  })
  it('system kind → system', () => {
    expect(resolveSource({ kind: 'system' }, live)).toEqual({ kind: 'system' })
  })
})

describe('normalizeLevel', () => {
  it('normalizes rms to 0..1 clamped', () => {
    expect(normalizeLevel(0)).toBe(0)
    expect(normalizeLevel(1)).toBe(1)            // 大訊號夾到 1
    expect(normalizeLevel(0.05)).toBeCloseTo(0.3, 1) // 小訊號線性放大（GAIN=6）
  })
})
