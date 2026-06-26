import { describe, it, expect } from 'vitest'
import { maxIconKind } from '../player/windowIcons'

describe('maxIconKind', () => {
  it('最大化時為 restore', () => { expect(maxIconKind(true)).toBe('restore') })
  it('非最大化時為 maximize', () => { expect(maxIconKind(false)).toBe('maximize') })
})
