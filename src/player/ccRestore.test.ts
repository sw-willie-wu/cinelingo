import { describe, it, expect } from 'vitest'
import { pickCcRestore, clampSecondaryToPrimary } from './ccRestore'

const files = (...ids: string[]) => ids.map((id) => ({ id }))

describe('clampSecondaryToPrimary', () => {
  it('primary off → secondary 強制 off', () => {
    expect(clampSecondaryToPrimary('off', 'f1')).toBe('off')
  })
  it('primary 非 off → secondary 不變', () => {
    expect(clampSecondaryToPrimary('live', 'f1')).toBe('f1')
    expect(clampSecondaryToPrimary('f2', 'off')).toBe('off')
  })
})

describe('pickCcRestore', () => {
  it('快照 primary 非 off 且有效 → 還原快照（次軌驗證）', () => {
    expect(pickCcRestore({ primary: 'f1', secondary: 'f2' }, files('f1','f2'), true))
      .toEqual({ primary: 'f1', secondary: 'f2' })
  })
  it('快照 live 但 master 關 → 跳過快照、退第一個檔', () => {
    expect(pickCcRestore({ primary: 'live', secondary: 'off' }, files('f1'), false))
      .toEqual({ primary: 'f1', secondary: 'off' })
  })
  it('快照 primary 指向不存在的檔 → 退第一個檔', () => {
    expect(pickCcRestore({ primary: 'fX', secondary: 'off' }, files('f1'), true))
      .toEqual({ primary: 'f1', secondary: 'off' })
  })
  it('無快照、有檔 → 第一個檔', () => {
    expect(pickCcRestore(null, files('fa','fb'), true)).toEqual({ primary: 'fa', secondary: 'off' })
  })
  it('無快照、無檔、master 開 → live', () => {
    expect(pickCcRestore(null, files(), true)).toEqual({ primary: 'live', secondary: 'off' })
  })
  it('無快照、無檔、master 關 → null（no-op）', () => {
    expect(pickCcRestore(null, files(), false)).toBeNull()
  })
  it('快照次軌指向不存在檔 → 次軌降 off', () => {
    expect(pickCcRestore({ primary: 'f1', secondary: 'fGone' }, files('f1'), true))
      .toEqual({ primary: 'f1', secondary: 'off' })
  })
})
