import { reactive } from 'vue'
import { listen } from '@tauri-apps/api/event'
import { checkYtdlp, provisionYtdlp } from './backend'
import { useSettings } from './useSettings'

type Phase = 'idle' | 'checking' | 'downloading' | 'ready' | 'error'
const state = reactive<{ phase: Phase; pct: number; error: string }>({ phase: 'idle', pct: 0, error: '' })

let wired = false
function ensureWired() {
  if (wired) return
  wired = true
  listen<{ phase: string; done: number; total: number | null }>('sub-progress', (e) => {
    if (e.payload.phase !== 'ytdlp') return
    state.phase = 'downloading'
    state.pct = e.payload.total ? Math.round((e.payload.done / e.payload.total) * 100) : 0
  })
}

/** 開關轉 ON：檢查 yt-dlp，缺則下載。成功 → enabled=true + ready；失敗 → error（開關回 false）。 */
async function enable(): Promise<void> {
  if (state.phase === 'checking' || state.phase === 'downloading') return // 防重入：下載中再點開關不重複下載（避免兩個 provisionYtdlp 撞同一 .part）
  ensureWired()
  const s = useSettings().state
  state.error = ''; state.phase = 'checking'
  try {
    if (!(await checkYtdlp())) { state.phase = 'downloading'; await provisionYtdlp() }
    state.phase = 'ready'; s.capture.enabled = true
  } catch (e) {
    state.phase = 'error'; state.error = String(e); s.capture.enabled = false
  }
}
function disable(): void { useSettings().state.capture.enabled = false; state.phase = 'idle' }
function retry(): void { void enable() }

export function useCaptureProvision() { return { state, enable, disable, retry } }
