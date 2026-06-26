import { describe, it, expect, vi } from 'vitest'

// 測試檔在 app/src/__tests__/ → 相對源用 ../player/（對齊既有 remoteUrl.test.ts:2 / drop.test.ts:2）
vi.mock('../player/backend', () => ({
  loadRecent: vi.fn(async () => []),
  saveRecent: vi.fn(async () => {}),
  pathExists: vi.fn(async () => true),
}))

import { coerceRecent, dedupCap, useRecent } from '../player/useRecent'
import type { RecentItem } from '../player/recentTypes'

const mk = (id: string, t = 1): RecentItem => ({ kind: 'local', id, title: id, lastPlayedAt: t })

describe('coerceRecent', () => {
  it('丟壞形狀、留好項', () => {
    const raw = [mk('a'), { id: 1 }, null, { kind: 'remote', id: 'b', title: 'B', lastPlayedAt: 2 }]
    const r = coerceRecent(raw)
    expect(r.map((x) => x.id)).toEqual(['a', 'b'])
  })
  it('非陣列 → 空', () => {
    expect(coerceRecent({})).toEqual([])
  })
})

describe('dedupCap', () => {
  it('依 id 去重移頂、cap 50', () => {
    const list = [mk('a', 1), mk('b', 2)]
    const r = dedupCap(list, mk('a', 9))
    expect(r[0]).toMatchObject({ id: 'a', lastPlayedAt: 9 })
    expect(r.length).toBe(2)
  })
  it('cap 50 截尾', () => {
    let list: RecentItem[] = []
    for (let i = 0; i < 50; i++) list = dedupCap(list, mk('id' + i, i))
    list = dedupCap(list, mk('new', 99))
    expect(list.length).toBe(50)
    expect(list[0].id).toBe('new')
  })
})

describe('useRecent remove / clear', () => {
  it('remove 依 id 刪單個', () => {
    const r = useRecent()
    r.items.splice(0, r.items.length, mk('a'), mk('b'), mk('c'))
    r.remove('b')
    expect(r.items.map((x) => x.id)).toEqual(['a', 'c'])
  })
  it('clear 清空全部', () => {
    const r = useRecent()
    r.items.splice(0, r.items.length, mk('a'), mk('b'))
    r.clear()
    expect(r.items.length).toBe(0)
  })
})
