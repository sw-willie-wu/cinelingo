/** 軌道與檔案記憶的純函式（與後端 cache::normalize / Part C 快取鍵規則等價）。 */

export interface SubFile { id: string; path: string | null }
export interface StoredTrack { source: string; delaySec: number }
export interface StoredEntry { manualFiles: string[]; primary: StoredTrack; secondary: StoredTrack }

/** 正規化路徑作 KV key：小寫 + 統一分隔符。Windows 大小寫不敏感 → 同片同鍵；與 Rust normalize 等價。 */
export function normKey(path: string): string {
  return path.toLowerCase().replace(/\\/g, '/')
}

/** runtime track（source 可能是 fileId）→ 可儲存格式：fileId → 該檔絕對 path；'off'/'live' 原樣。 */
export function trackToStored(track: StoredTrack, files: SubFile[]): StoredTrack {
  const f = files.find((x) => x.id === track.source)
  return { source: f?.path ?? track.source, delaySec: track.delaySec }
}

/** 儲存的 source → runtime source：'off'→off；'live'→master 開才 live 否則 off；path→對應 fileId 否則 off。 */
export function restoreTrackSource(storedSource: string, files: SubFile[], masterOn: boolean): string {
  if (storedSource === 'off') return 'off'
  if (storedSource === 'live') return masterOn ? 'live' : 'off'
  return files.find((f) => f.path === storedSource)?.id ?? 'off'
}

/** 把從磁碟讀回的任意 raw 值強制塑型為完整的 StoredEntry（缺欄位 → 安全預設值）。純函式、無副作用。 */
function coerceTrack(t: unknown): StoredTrack {
  if (t !== null && typeof t === 'object') {
    const obj = t as Record<string, unknown>
    return {
      source: typeof obj.source === 'string' ? obj.source : 'off',
      delaySec: typeof obj.delaySec === 'number' ? obj.delaySec : 0,
    }
  }
  return { source: 'off', delaySec: 0 }
}

export function coerceStoredEntry(raw: unknown): StoredEntry {
  const r = raw !== null && typeof raw === 'object' ? (raw as Record<string, unknown>) : {}
  return {
    manualFiles: Array.isArray(r.manualFiles)
      ? (r.manualFiles as unknown[]).filter((x): x is string => typeof x === 'string')
      : [],
    primary: coerceTrack(r.primary),
    secondary: coerceTrack(r.secondary),
  }
}
