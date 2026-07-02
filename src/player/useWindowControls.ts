import { ref, readonly } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { LogicalSize } from '@tauri-apps/api/dpi'

const win = getCurrentWebviewWindow()

const isMaximized = ref(false)
const alwaysOnTop = ref(false)
let started = false

const SIZE_KEY = 'cinelingo:window-size'

function logErr(label: string) {
  return (e: unknown) => console.error(`[window] ${label} failed`, e)
}

async function syncMaximized(): Promise<void> {
  try { isMaximized.value = await win.isMaximized() } catch (e) { logErr('isMaximized')(e) }
}

async function saveWindowSize(): Promise<void> {
  try {
    if (await win.isFullscreen()) return
    const size = await win.innerSize()
    const factor = await win.scaleFactor()
    const logical = size.toLogical(factor)
    localStorage.setItem(SIZE_KEY, JSON.stringify({ w: Math.round(logical.width), h: Math.round(logical.height) }))
  } catch (e) { logErr('saveWindowSize')(e) }
}

async function restoreWindowSize(): Promise<void> {
  try {
    const raw = localStorage.getItem(SIZE_KEY)
    if (!raw) return
    const { w, h } = JSON.parse(raw) as { w: number; h: number }
    if (w > 200 && h > 150) await win.setSize(new LogicalSize(w, h))
  } catch (e) { logErr('restoreWindowSize')(e) }
}

async function start(): Promise<void> {
  if (started) return                 // init-once:onResized 只註冊一次
  started = true
  await restoreWindowSize()           // 套用上次記憶的大小（覆蓋 tauri.conf.json 預設值）
  await syncMaximized()               // 初始 seed 圖示
  // Tauri 無 maximize 事件;最大化/還原/snap 都會改變尺寸 → onResized 後重讀
  await win.onResized(async () => {
    await syncMaximized()
    if (!isMaximized.value) await saveWindowSize()
  })
}

async function minimize(): Promise<void> {
  await win.minimize().catch(logErr('minimize'))
}

async function toggleMaximize(): Promise<void> {
  try {
    if (await win.isMaximized()) await win.unmaximize()
    else await win.maximize()
    await syncMaximized()
  } catch (e) { logErr('toggleMaximize')(e) }
}

async function close(): Promise<void> {
  // close() 會發 closeRequested → App.vue 既有監聽做 mpv 清理;不可用 destroy()
  await win.close().catch(logErr('close'))
}

async function toggleAlwaysOnTop(): Promise<void> {
  const next = !alwaysOnTop.value
  try {
    await win.setAlwaysOnTop(next)
    alwaysOnTop.value = next
  } catch (e) { logErr('alwaysOnTop')(e) }
}

export function useWindowControls() {
  return {
    isMaximized: readonly(isMaximized),
    alwaysOnTop: readonly(alwaysOnTop),
    start, minimize, toggleMaximize, close, toggleAlwaysOnTop,
  }
}
