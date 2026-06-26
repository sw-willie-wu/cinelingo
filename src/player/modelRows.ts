import type { ModelKey } from './settings'

export type RowState = 'active' | 'downloaded' | 'downloading' | 'error' | 'idle'

/** 列狀態優先序：downloading > error > (downloaded ? active/downloaded) > idle。 */
export function rowState(
  key: ModelKey,
  selectedModel: ModelKey,
  downloaded: Set<string>,
  downloading: Set<string>,
  errored: Set<string>,
): RowState {
  if (downloading.has(key)) return 'downloading'
  if (errored.has(key)) return 'error'
  if (downloaded.has(key)) return key === selectedModel ? 'active' : 'downloaded'
  return 'idle'
}

/** 下載完成後是否要自動選取。轉寫中一律 null（不打斷正在進行的轉寫/下載）。 */
export function nextAutoSelect(
  selectedModel: ModelKey,
  downloaded: Set<string>,
  justDownloaded: ModelKey,
  transcribing: boolean,
): ModelKey | null {
  if (transcribing) return null
  return downloaded.has(selectedModel) ? null : justDownloaded
}
