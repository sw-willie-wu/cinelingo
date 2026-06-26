import { describe, it, expect } from 'vitest'
import { RESIZE_HANDLES } from '../player/resize'

const VALID = new Set([
  'North', 'South', 'East', 'West',
  'NorthEast', 'NorthWest', 'SouthEast', 'SouthWest',
])

describe('RESIZE_HANDLES', () => {
  it('共 8 個把手', () => { expect(RESIZE_HANDLES).toHaveLength(8) })
  it('方向皆為合法 Tauri ResizeDirection(單字 PascalCase)', () => {
    for (const h of RESIZE_HANDLES) expect(VALID.has(h.dir)).toBe(true)
  })
  it('涵蓋全部 8 個方向、不重複', () => {
    const dirs = RESIZE_HANDLES.map((h) => h.dir)
    expect(new Set(dirs).size).toBe(8)
  })
  it('key 不重複', () => {
    const keys = RESIZE_HANDLES.map((h) => h.key)
    expect(new Set(keys).size).toBe(keys.length)
  })
})
