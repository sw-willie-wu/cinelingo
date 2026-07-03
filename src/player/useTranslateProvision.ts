import { reactive } from 'vue'
import { checkTranslateEngine, provisionTranslateRuntime } from './backend'
import { useSettings, whenSettingsReady } from './useSettings'

/** runtime(llmServer) 缺 → 'ask'；否則 → 'enable'（llmModel 不 gate 啟用）。純函式（供測 + requestEnable）。 */
export function decideTranslateEnable(missing: { kind: string }[]): 'ask' | 'enable' {
  return missing.some((m) => m.kind === 'llmServer') ? 'ask' : 'enable'
}

type Phase = 'ask' | 'downloading' | 'error'
const state = reactive<{ open: boolean; phase: Phase; missing: { kind: string; sizeMb: number }[]; totalMb: number; error: string }>(
  { open: false, phase: 'ask', missing: [], totalMb: 0, error: '' })

function enable(): void { useSettings().state.liveSubs.translateEnabled = true }
function close(): void { state.open = false }

/** master ON：runtime 缺 → 詢問對話框；runtime 齊 → 直接啟用。llmModel 不 gate 啟用。 */
async function requestEnable(): Promise<void> {
  const key = useSettings().state.liveSubs.translateModel
  let missing: { kind: string; sizeMb: number }[]
  try { missing = await checkTranslateEngine(key) } catch (e) { state.error = String(e); state.phase = 'error'; state.open = true; return }
  if (decideTranslateEnable(missing) === 'enable') { enable(); return } // runtime 齊（含只缺 llmModel）→ 直接啟用
  const runtime = missing.filter((m) => m.kind === 'llmServer')
  state.missing = runtime; state.totalMb = runtime.reduce((s, m) => s + m.sizeMb, 0); state.error = ''; state.phase = 'ask'; state.open = true
}

/** [是]：裝 runtime（不可關）→ 成功啟用 / 失敗轉 error。 */
async function confirm(): Promise<void> {
  state.phase = 'downloading'
  try { await provisionTranslateRuntime(); enable(); close() } catch (e) { state.error = String(e); state.phase = 'error' }
}
function cancel(): void { if (state.phase === 'downloading') return; close() }
function retry(): void { void requestEnable() }
function disable(): void { useSettings().state.liveSubs.translateEnabled = false }

/** 啟動再確認：已啟用但 runtime 缺 → 降級 + 詢問（只看 llmServer，無模型不降級）。 */
async function verifyTranslateOnStartup(): Promise<void> {
  await whenSettingsReady()
  const s = useSettings().state
  if (!s.liveSubs.translateEnabled) return
  let missing: { kind: string; sizeMb: number }[]
  try { missing = await checkTranslateEngine(s.liveSubs.translateModel) } catch { return }
  if (!missing.some((m) => m.kind === 'llmServer')) return
  s.liveSubs.translateEnabled = false
  const runtime = missing.filter((m) => m.kind === 'llmServer')
  state.missing = runtime; state.totalMb = runtime.reduce((sum, m) => sum + m.sizeMb, 0); state.error = ''; state.phase = 'ask'; state.open = true
}

export function useTranslateProvision() {
  return { state, requestEnable, confirm, cancel, retry, disable, verifyTranslateOnStartup }
}
