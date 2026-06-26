import { describe, it, expect } from 'vitest'
import { resolveSource } from './useAudioSource'
import type { AudioSources } from './backend'

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
