import { reactive, computed } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { listModels, downloadModel, listTranslateModels } from './backend'
import { useSettings } from './useSettings'
import { useSubtitles } from './useSubtitles'
import { nextAutoSelect, nextAutoSelectTranslate } from './modelRows'
import type { ModelKey, TranslateModelKey } from './settings'

interface ModelDownloadEvent {
  key: string
  phase: 'downloading' | 'done' | 'error'
  done: number
  total: number | null
  message: string | null
}

// 真實 whisper 模型 key（自動選模只對這些有效；LLM 引擎下載的 key 不算）。
export const WHISPER_MODEL_KEYS = new Set<string>(['small', 'medium', 'turbo', 'large-v3'])
const TRANSLATE_MODEL_KEYS = new Set<string>(['translate-4b', 'translate-12b'])

// 模組層單例（跨設定 Modal 開/關存活；背景下載中關掉再開仍見進度）。
const downloaded = reactive(new Set<string>())
const downloading = reactive(new Map<string, { done: number; total: number | null }>())
const errored = reactive(new Set<string>())

/** downloaded 是否含任一真實 whisper 模型 key。純函式（供測 + hasWhisperModel）。 */
export function computeHasWhisperModel(dl: Set<string>): boolean {
  for (const k of WHISPER_MODEL_KEYS) if (dl.has(k)) return true
  return false
}
const hasWhisperModel = computed(() => computeHasWhisperModel(downloaded))

let wired: Promise<void> | null = null

function ensureWired(): Promise<void> {
  if (!wired) {
    wired = (async () => {
      try {
        for (const m of await listModels()) if (m.downloaded) downloaded.add(m.key)
      } catch {
        /* 視為皆未下載 */
      }
      try {
        for (const m of await listTranslateModels()) if (m.downloaded) downloaded.add(m.key)
      } catch { /* ignore */ }
      await listen<ModelDownloadEvent>('model-download', (e) => {
        const { key, phase, done, total } = e.payload
        if (phase === 'downloading') {
          downloading.set(key, { done, total })
          errored.delete(key)
        } else if (phase === 'done') {
          downloading.delete(key)
          downloaded.add(key)
          errored.delete(key)
          // 只對「真實 whisper 模型 key」做自動選模；其他 key（翻譯模型 'translate-4b'/
          // 'translate-12b' 見下方分支，共用引擎 'llm-server'）也會走 model-download 'done'，
          // 但 `key as ModelKey` 對它們是不安全轉型，不可拿去寫 liveSubs.model。
          if (WHISPER_MODEL_KEYS.has(key)) {
            const settings = useSettings()
            const pick = nextAutoSelect(
              settings.state.liveSubs.model,
              downloaded,
              key as ModelKey,
              useSubtitles().enabled.value,
            )
            if (pick) settings.state.liveSubs.model = pick
          }
          if (TRANSLATE_MODEL_KEYS.has(key)) {
            const settings = useSettings()
            const pick = nextAutoSelectTranslate(
              settings.state.liveSubs.translateModel,
              downloaded,
              key as TranslateModelKey,
            )
            if (pick) settings.state.liveSubs.translateModel = pick
          }
        } else {
          downloading.delete(key)
          errored.add(key)
        }
      })
    })()
  }
  return wired
}

// 樂觀設定 → 即時回饋（含手動成等待者時）；手動路徑必跑完並 emit terminal → 必清（孤兒安全）。
async function download(key: string): Promise<void> {
  await ensureWired() // 確保 model-download listener 已掛、downloaded 已載入，再下載（修首開競態，避免瞬間 done 事件遺失）
  if (downloaded.has(key) || downloading.has(key)) return
  downloading.set(key, { done: 0, total: null })
  try {
    await downloadModel(key)
  } catch {
    // 正常失敗已由 error 事件標記；但若指令在 emit 任何事件前就 reject（如 app_data_dir 失敗），
    // 樂觀項目會無人清 → 在此自清，避免該列卡「下載中…」（孤兒安全備援）。
    if (downloading.has(key)) {
      downloading.delete(key)
      errored.add(key)
    }
  }
}

export function useModelDownloads() {
  ensureWired()
  return { downloaded, downloading, errored, download, hasWhisperModel }
}
