import { ref, readonly } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'

const win = getCurrentWebviewWindow()

const isMaximized = ref(false)
const alwaysOnTop = ref(false)
let started = false

function logErr(label: string) {
  return (e: unknown) => console.error(`[window] ${label} failed`, e)
}

async function syncMaximized(): Promise<void> {
  try { isMaximized.value = await win.isMaximized() } catch (e) { logErr('isMaximized')(e) }
}

async function start(): Promise<void> {
  if (started) return                 // init-once:onResized 只註冊一次
  started = true
  await syncMaximized()               // 初始 seed 圖示
  // Tauri 無 maximize 事件;最大化/還原/snap 都會改變尺寸 → onResized 後重讀
  await win.onResized(() => { syncMaximized() })
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
