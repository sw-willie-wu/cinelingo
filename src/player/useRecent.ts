import { reactive } from 'vue'
import { loadRecent, saveRecent, pathExists } from './backend'
import type { RecentItem } from './recentTypes'

const CAP = 50

export function coerceRecent(raw: unknown): RecentItem[] {
  if (!Array.isArray(raw)) return []
  const out: RecentItem[] = []
  for (const e of raw) {
    if (!e || typeof e !== 'object') continue
    const o = e as Record<string, unknown>
    if ((o.kind !== 'local' && o.kind !== 'remote') || typeof o.id !== 'string' || typeof o.title !== 'string') continue
    out.push({
      kind: o.kind,
      id: o.id,
      title: o.title,
      lastPlayedAt: typeof o.lastPlayedAt === 'number' ? o.lastPlayedAt : 0,
    })
  }
  return out
}

export function dedupCap(list: RecentItem[], item: RecentItem): RecentItem[] {
  const rest = list.filter((x) => x.id !== item.id)
  return [item, ...rest].slice(0, CAP)
}

const items = reactive<RecentItem[]>([])
let saveTimer: ReturnType<typeof setTimeout> | null = null
let loadPromise: Promise<void> | null = null

// 讀盤一次（memoized）：所有寫入路徑都先 await 這個，確保 disk 既有歷史已載入後才動 items，
// 否則「未開面板就先播」會在空 items 上 record → debounce 覆蓋掉 recent.json 既有歷史（資料遺失）。
function load(): Promise<void> {
  if (!loadPromise) {
    loadPromise = loadRecent()
      .catch(() => [])
      .then((raw) => { items.splice(0, items.length, ...coerceRecent(raw)) })
  }
  return loadPromise
}

function scheduleSave(): void {
  if (saveTimer) clearTimeout(saveTimer)
  saveTimer = setTimeout(() => {
    saveRecent(items.slice()).catch(() => {})
    saveTimer = null
  }, 300)
}

export function useRecent() {
  async function record(item: { kind: 'local' | 'remote'; id: string; title: string }): Promise<void> {
    await load()                       // 先讀盤再寫 → 不覆蓋既有歷史
    const next = dedupCap(items.slice(), { ...item, lastPlayedAt: Date.now() })
    items.splice(0, items.length, ...next)
    scheduleSave()
  }
  async function refreshMissing(): Promise<void> {
    await load()
    for (const it of items) {
      if (it.kind === 'local') it.missing = !(await pathExists(it.id))
    }
  }
  function remove(id: string): void {
    const i = items.findIndex((x) => x.id === id)
    if (i >= 0) { items.splice(i, 1); scheduleSave() }
  }
  function clear(): void {
    if (items.length === 0) return
    items.splice(0, items.length)
    scheduleSave()
  }
  return { items, load, record, refreshMissing, remove, clear }
}
