import { ref } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { availableMonitors } from '@tauri-apps/api/window'
import { PhysicalSize, PhysicalPosition } from '@tauri-apps/api/dpi'
import { setVideoHidden } from '../mpv'
import { usePlayer } from './usePlayer'
import { useSubtitles } from './useSubtitles'
import { useSettings } from './useSettings'
import { scaleFontPx } from './settings'
import { clampToVisible, defaultFloatingRect, captionBarHeight, type Mon, type Rect } from './floatingLayout'

// 浮動條渲染字級＝依「螢幕高度」等比（與 FloatingCaptions 一致）→ 條高用它算才不切字。
const floatingFontPx = (fontSize: number): number => scaleFontPx(fontSize, (typeof window !== 'undefined' && window.screen?.height) || 1080)

const active = ref(false)
const hovering = ref(false)   // 由 FloatingCaptions 的 mouseenter/leave 驅動（無穿透 → JS 直接收滑鼠事件）
let transitioning = false
let savedRect: { x: number; y: number; width: number; height: number } | null = null
let startedLoopback = false   // 本次是否由我們啟動 loopback（沿用既有字幕時為 false → 退出不要去停它）

async function monitors(): Promise<Mon[]> {
  const ms = await availableMonitors()
  return ms.map((m) => ({ x: m.position.x, y: m.position.y, width: m.size.width, height: m.size.height }))
}

async function applyRect(win: ReturnType<typeof getCurrentWebviewWindow>, rect: Rect, height: number): Promise<void> {
  await win.setSize(new PhysicalSize(rect.width, height))
  await win.setPosition(new PhysicalPosition(rect.x, rect.y))
}

async function enter(): Promise<void> {
  if (active.value || transitioning) return                 // step 0 再入/轉場守衛
  const settings = useSettings().state
  const player = usePlayer()
  const subs = useSubtitles()
  // 已有可沿用的時鐘字幕（字幕檔 / mode A 即時字幕）→ 直接沿用，不開 loopback、也不需引擎/語言守衛。
  const reuse = subs.tracks.primary.source !== 'off'
  if (!reuse) {
    if (!settings.liveSubs.enabled) {                        // 守衛 A：引擎備妥（僅 loopback 需要）
      player.notify('請先在設定→即時字幕啟用並下載引擎'); return
    }
    if (settings.liveSubs.sourceLang === 'auto') {          // 守衛 B：明確語言（僅 loopback 需要）
      player.notify('請先在設定→即時字幕將來源語言設為明確語言'); return
    }
  }
  transitioning = true
  startedLoopback = false
  const win = getCurrentWebviewWindow()
  try {
    const pos = await win.outerPosition(); const size = await win.outerSize()
    savedRect = { x: pos.x, y: pos.y, width: size.width, height: size.height }
    if (await win.isFullscreen()) await win.setFullscreen(false)
    await setVideoHidden(true)   // mpv 只播音訊、放掉影像視窗 → 透明（不 destroy，音訊連續）
    active.value = true
    const height = captionBarHeight(settings.liveSubs.display.lines, floatingFontPx(settings.appearance.primary.fontSize))
    const ms = await monitors()
    const f = settings.floating
    const rect: Rect = (f.x != null && f.y != null && f.width != null)
      ? clampToVisible({ x: f.x, y: f.y, width: f.width }, height, ms)
      : defaultFloatingRect(height, ms)
    await applyRect(win, rect, height)
    await win.setAlwaysOnTop(true)
    await win.setSkipTaskbar(true)
    await win.setShadow(false)   // 關掉 Windows borderless 視窗的 DWM 陰影/邊框（細條浮動字幕才不露出框）
    if (!reuse) { await subs.startLoopbackCapture(null); startedLoopback = true }  // 沿用既有字幕時不開 loopback
  } catch (e) {
    // 不提前清 transitioning：rollback 期間保持 true 擋住 Esc 並行 exit（避免雙重 restart）。
    await rollback()
    usePlayer().notify('啟動浮動字幕失敗：' + String(e))
    return
  } finally {
    transitioning = false
  }
}

/** 還原視窗 + 影像回來（exit 與 enter 失敗回滾共用）。 */
async function rollback(): Promise<void> {
  const win = getCurrentWebviewWindow()
  hovering.value = false
  if (startedLoopback) { try { await useSubtitles().stopLoopbackCapture() } catch { /* */ } }  // 沿用既有字幕時不停它
  startedLoopback = false
  try { await win.setAlwaysOnTop(false); await win.setSkipTaskbar(false); await win.setShadow(true) } catch { /* */ }
  if (savedRect) { try { await applyRect(win, savedRect, savedRect.height) } catch { /* */ } }
  active.value = false
  // 影像回來（mpv 一直活著、音訊連續 → 不需 restart、無 auto-play 殘留問題）。
  try { await setVideoHidden(false) } catch (e) { usePlayer().notify('還原影像失敗，請重啟應用'); console.error(e) }
  savedRect = null
}

async function exit(): Promise<void> {
  if (!active.value || transitioning) return                 // 冪等 + 轉場中不打斷 enter
  try {
    const win = getCurrentWebviewWindow()
    const pos = await win.outerPosition(); const size = await win.outerSize()
    persistFloating(pos.x, pos.y, size.width)
  } catch { /* */ }
  await rollback()
}

function persistFloating(x: number, y: number, width: number): void {
  // useSettings 無顯式 save()；持久化靠 state deep watch（300ms debounce）。直接 mutate reactive state。
  useSettings().state.floating = { x, y, width }
}

/** 測試輔助：重置單例狀態。 */
function _forceReset(): void { active.value = false; transitioning = false; savedRect = null; hovering.value = false }

export function useFloatingMode() {
  return { active, hovering, enter, exit, toggle: () => (active.value ? exit() : enter()), _forceReset }
}
