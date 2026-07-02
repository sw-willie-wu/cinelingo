import { describe, it, expect } from 'vitest'
import { trackToStored, coerceStoredEntry } from '../player/subMemory'
describe('subMemory per-track translateTo', () => {
  it('round-trips translateTo through store + coerce', () => {
    const stored = trackToStored({ source: 'live', delaySec: 0.5, translateTo: 'zh-Hant' }, [])
    expect(stored.translateTo).toBe('zh-Hant')
    const entry = coerceStoredEntry({ manualFiles: [], primary: stored, secondary: { source: 'off', delaySec: 0 } })
    expect(entry.primary.translateTo).toBe('zh-Hant')
    expect(entry.secondary.translateTo).toBe('off') // missing → default
  })
})
