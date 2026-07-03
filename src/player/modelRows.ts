import type { ModelKey, TranslateModelKey } from './settings'

export type RowState = 'active' | 'downloaded' | 'downloading' | 'error' | 'idle'

/** 列狀態優先序：downloading > error > (downloaded ? active/downloaded) > idle。 */
export function rowState(
  key: string,
  selectedModel: string,
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

/** 翻譯模型下載完成後是否自動選取（目前選取的未下載才切換）。 */
export function nextAutoSelectTranslate(
  selected: TranslateModelKey,
  downloaded: Set<string>,
  justDownloaded: TranslateModelKey,
): TranslateModelKey | null {
  return downloaded.has(selected) ? null : justDownloaded
}

/** 翻譯列提示三態：off→選來源／未啟用→啟用／啟用但無模型→下載模型／可翻→空。純函式。 */
export function transHintText(source: string, translateEnabled: boolean, modelDownloaded: boolean): string {
  if (source === 'off') return '需先選字幕來源'
  if (!translateEnabled) return '需在設定啟用翻譯'
  if (!modelDownloaded) return '需在設定下載翻譯模型'
  return ''
}
