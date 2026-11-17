import { reactive } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { checkYtdlpUpdate, updateYtdlp } from './backend'
import { useSettings, whenSettingsReady } from './useSettings'
import { useEngineProvision } from './useEngineProvision'
import { useTranslateProvision } from './useTranslateProvision'

type Phase = 'ask' | 'downloading' | 'done' | 'error'
const state = reactive<{
  open: boolean; phase: Phase; local: string; latest: string; version: string
  error: string; pct: number; doneMb: number; totalMb: number
}>({ open: false, phase: 'ask', local: '', latest: '', version: '', error: '', pct: 0, doneMb: 0, totalMb: 0 })

let listening = false

/** 啟動檢查：yt-dlp 有新版 → 開詢問對話框。全程吞錯（不阻塞啟動）。 */
async function checkOnStartup(): Promise<void> {
  try {
    await whenSettingsReady()
    if (!useSettings().state.capture.enabled) return
    const info = await checkYtdlpUpdate()
    if (!info.outdated) return
    if (useEngineProvision().state.open || useTranslateProvision().state.open) return
    state.local = info.local
    state.latest = info.latest
    state.error = ''
    state.pct = 0
    state.phase = 'ask'
    state.open = true
  } catch (e) {
    console.warn('[ytdlp-update] check failed', e)
  }
}

/** 〔更新〕與〔重試〕共用。downloading 中忽略（防重入，須在任何 await 前同步設 phase）。 */
async function update(): Promise<void> {
  if (state.phase === 'downloading') return
  state.phase = 'downloading'
  state.error = ''
  state.pct = 0
  state.doneMb = 0
  state.totalMb = 0
  if (!listening) {
    listening = true
    try {
      await listen<{ phase: string; done: number; total: number | null }>('sub-progress', (e) => {
        if (e.payload.phase !== 'ytdlp-update') return
        const { done, total } = e.payload
        state.doneMb = Math.round(done / 1e6)
        state.totalMb = total ? Math.round(total / 1e6) : 0
        state.pct = total ? Math.round((done / total) * 100) : 0
      })
    } catch (e) {
      listening = false
      console.warn('[ytdlp-update] listen failed', e)
    }
  }
  try {
    state.version = await updateYtdlp()
    state.phase = 'done'
  } catch (e) {
    state.error = String(e)
    state.phase = 'error'
  }
}

function later(): void { if (state.phase === 'downloading') return; state.open = false }
function close(): void { if (state.phase === 'downloading') return; state.open = false }

export function useYtdlpUpdate() {
  return { state, checkOnStartup, update, later, close }
}
