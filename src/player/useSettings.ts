import { reactive, readonly, watch } from 'vue'
import { defaultSettings, mergeSettings, type Settings } from './settings'
import { loadSettingsRaw, saveSettingsRaw, detectHardware, type HwInfo } from './backend'

const state = reactive<Settings>(defaultSettings())
const hw = reactive<{ info: HwInfo | null }>({ info: null })
const modal = reactive<{ open: boolean }>({ open: false })

let initPromise: Promise<void> | null = null
function ensureInit(): Promise<void> {
  if (!initPromise) {
    initPromise = (async () => {
      // 1) 載入並合併（在掛 watch 前 → 載入本身不觸發存檔）
      let raw: unknown = {}
      try { raw = await loadSettingsRaw() } catch { raw = {} }
      Object.assign(state, mergeSettings(raw))
      // 2) 偵測硬體；解析 accelEnabled null → hasGpu
      try { hw.info = await detectHardware() } catch { hw.info = { backend: 'cpu', gpuName: null, hasGpu: false } }
      if (state.hardware.accelEnabled === null) state.hardware.accelEnabled = hw.info.hasGpu
      // 3) 之後變更 → debounce 存檔
      let t: ReturnType<typeof setTimeout> | null = null
      watch(state, () => {
        if (t) clearTimeout(t)
        t = setTimeout(() => {
          saveSettingsRaw(JSON.parse(JSON.stringify(state))).catch((e) => console.warn('[settings] save failed', e))
        }, 300)
      }, { deep: true })
    })()
  }
  return initPromise
}

export function useSettings() {
  ensureInit()
  return {
    state,                                  // reactive（面板以 v-model 直接寫入）
    hw: readonly(hw),
    modal,
    openModal: () => { modal.open = true },
    closeModal: () => { modal.open = false },
  }
}

/** 解析時機 = 設定已載入並合併、且存檔 watch 已掛上。供啟動流程等待。 */
export function whenSettingsReady(): Promise<void> {
  return ensureInit()
}
