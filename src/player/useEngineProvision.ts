import { reactive } from 'vue'
import { checkEngine, provisionEngine, type MissingAsset } from './backend'
import { provisionSummary, type ProvisionSummary } from './provision'
import { useSettings, whenSettingsReady } from './useSettings'
import { useSubtitles } from './useSubtitles'

type Phase = 'ask' | 'downloading' | 'error'
type From = 'check' | 'provision'

// 模組層單例：跨設定 Modal 開關存活。
const state = reactive<{
  open: boolean
  phase: Phase
  from: From
  missing: MissingAsset[]
  totalMb: number
  noModel: boolean
  error: string
}>({ open: false, phase: 'ask', from: 'check', missing: [], totalMb: 0, noModel: false, error: '' })

function enableNow(noModel: boolean): void {
  const s = useSettings().state
  s.liveSubs.enabled = true
  if (noModel) s.liveSubs.model = 'turbo' // 無模型 → 選 turbo
}

function close(): void {
  state.open = false
  useSubtitles().clearProgress() // 清殘留 sub-progress，避免 dlText 殘留
}

function openAsk(sum: ProvisionSummary): void {
  state.missing = sum.missing
  state.totalMb = sum.totalMb
  state.noModel = sum.noModel
  state.error = ''
  state.phase = 'ask'
  state.open = true
}

/** master toggle 轉 ON。全備妥 → 直接啟用；有缺 → 開「詢問」對話框。 */
async function requestEnable(): Promise<void> {
  try {
    const sum = provisionSummary(await checkEngine())
    if (sum.missing.length === 0) {
      enableNow(false)
      return
    }
    openAsk(sum)
  } catch (e) {
    state.from = 'check'
    state.error = String(e)
    state.phase = 'error'
    state.open = true
  }
}

/** [是]：下載（不可關閉）→ 成功啟用 / 失敗轉 error。 */
async function confirm(): Promise<void> {
  state.from = 'provision'
  state.phase = 'downloading'
  try {
    await provisionEngine()
    const noModel = state.noModel
    enableNow(noModel)
    close()
  } catch (e) {
    state.error = String(e)
    state.phase = 'error'
  }
}

/** [否] / error 的 [關閉]：下載中不可關。 */
function cancel(): void {
  if (state.phase === 'downloading') return
  close()
}

/** error 的 [重試]：依錯誤來源重跑 check 或 provision。 */
function retry(): void {
  if (state.from === 'check') void requestEnable()
  else void confirm()
}

/** master toggle 轉 OFF：停用功能 + 中止進行中的轉寫。 */
async function disableFeature(): Promise<void> {
  useSettings().state.liveSubs.enabled = false
  const subs = useSubtitles()
  if (subs.enabled.value) await subs.disable().catch(() => {}) // 旗標已設 false; 停轉寫失敗只是 console，吞掉避免 unhandled rejection
}

/** 啟動再確認：已啟用但缺依賴 → 降級 enabled + 跳備妥「詢問」對話框（自動重下載）。 */
async function verifyOnStartup(): Promise<void> {
  await whenSettingsReady()
  const s = useSettings().state
  if (!s.liveSubs.enabled) return // 未啟用 → 無需驗證
  let sum: ProvisionSummary
  try {
    sum = provisionSummary(await checkEngine())
  } catch {
    return // 無法驗證 → 不打擾啟動（轉寫安全網仍在）
  }
  if (sum.missing.length === 0) return // 依賴齊全 → 維持啟用
  s.liveSubs.enabled = false // 缺依賴 → 誠實降級為未啟用
  openAsk(sum)
}

export function useEngineProvision() {
  return { state, requestEnable, confirm, cancel, retry, disableFeature, verifyOnStartup }
}
