<script setup lang="ts">
import { onMounted, onBeforeUnmount, watch, computed, ref } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { revealItemInDir } from '@tauri-apps/plugin-opener'
import { usePlayer } from './player/usePlayer'
import { useKeyboard } from './player/useKeyboard'
import { useAutoHide } from './player/useAutoHide'
import { useWindowControls } from './player/useWindowControls'
import { useSubtitles } from './player/useSubtitles'
import { useQueue } from './player/useQueue'
import { expandPlayablePaths } from './player/backend'
import { basename } from './player/path'
import type { QueueItem } from './player/queueTypes'
import TitleBar from './components/TitleBar.vue'
import ControlBar from './components/ControlBar.vue'
import ResizeHandles from './components/ResizeHandles.vue'
import SubtitleOverlay from './components/SubtitleOverlay.vue'
import SettingsModal from './components/SettingsModal.vue'
import EngineProvisionDialog from './components/EngineProvisionDialog.vue'
import { useSettings } from './player/useSettings'
import { useEngineProvision } from './player/useEngineProvision'
import { setVideoBlur } from './mpv'
import { usePasteUrl } from './player/usePasteUrl'
import { useAudioSource } from './player/useAudioSource'
import { useFloatingMode } from './player/useFloatingMode'
import FloatingCaptions from './components/FloatingCaptions.vue'
import AudioVisualizer from './components/AudioVisualizer.vue'
import LoadingOverlay from './components/LoadingOverlay.vue'
import * as playbackMemory from './player/playbackMemory'
import wordmarkUrl from './assets/cinelingo-wordmark.png'

const player = usePlayer()
const queue = useQueue()
const { visible, setPointerOverBar } = useAutoHide()
const windowControls = useWindowControls()
const subs = useSubtitles()
const settings = useSettings()
const floating = useFloatingMode()
const audioSource = useAudioSource()
const normalMode = computed(() => !floating.active.value)  // 非浮動模式（正常播放器版面）
useKeyboard()
usePasteUrl()

// 空狀態：未載入任何來源、非解析中、且未 armed（armed 時顯示 AudioVisualizer 取代）。
const showEmptyState = computed(() => !player.state.path && !player.source.resolving && !audioSource.armed.value)

// 換檔 → AI 字幕重啟(啟用中)或清舊 cue；套 per-video speed/audio-delay。
watch(() => player.state.path, async () => {
  await subs.onFileChanged().catch((e) => console.warn('[app] onFileChanged', e))
  const id = player.source.current?.id ?? ''
  await playbackMemory.applyForCurrent(id).catch((e) => console.warn('[app] applyForCurrent', e))
})

// 設定 Modal 開關 → 對影片套/移除高斯模糊（磨砂玻璃背景）。
watch(() => settings.modal.open, (open) => { setVideoBlur(open) })

// 下載引擎進度 banner 文字(僅「轉寫進行中」的資產補抓階段顯示；
// 備妥流(master toggle / 啟動驗證)時 subs.enabled=false → 不顯示, 由 EngineProvisionDialog 自己呈現進度,
// 也避免 provision 尾隨 sub-progress 賽過 clearProgress 後殘留 banner)。
const dlText = computed(() => {
  if (!subs.enabled.value) return null
  const p = subs.progress.value
  if (!p || !['model', 'vad', 'backend', 'ffmpeg'].includes(p.phase)) return null
  const mb = (n: number) => Math.round(n / 1e6)
  return `下載引擎… ${p.phase} ${mb(p.done)}MB${p.total ? '/' + mb(p.total) + 'MB' : ''}`
})

let unlistenDrop: (() => void) | null = null
let unlistenClose: (() => void) | null = null
let unlistenRec: (() => void) | null = null

// 錄音存檔提示（點擊開啟資料夾）；後端 disarm/重 arm 時 emit recording-saved。
const recordingSaved = ref<string | null>(null)
let recTimer: ReturnType<typeof setTimeout> | undefined
function openRecordingFolder() { if (recordingSaved.value) void revealItemInDir(recordingSaved.value) }

onMounted(async () => {
  await player.start()
  await windowControls.start()
  void useEngineProvision().verifyOnStartup()
  const win = getCurrentWebviewWindow()

  unlistenDrop = await win.onDragDropEvent(async (ev) => {
    if (ev.payload.type === 'drop') {
      let paths: string[]
      try { paths = await expandPlayablePaths(ev.payload.paths) }  // 後端展開檔/資料夾、自然排序
      catch { player.notify('無法讀取拖入的項目'); return }
      if (paths.length === 0) { player.notify('沒有可播放的檔案'); return }
      const items: QueueItem[] = paths.map((p) => ({ kind: 'local', id: p, title: basename(p) }))
      await queue.enqueueItems(items, { noAutoplay: useAudioSource().armed.value })  // armed→不自動播；否則空佇列→播第一支
    }
  })

  unlistenRec = await listen<string>('recording-saved', (e) => {
    recordingSaved.value = e.payload
    clearTimeout(recTimer)
    recTimer = setTimeout(() => { recordingSaved.value = null }, 6000)
  })

  unlistenClose = await win.onCloseRequested(async (event) => {
    event.preventDefault()
    try {
      await subs.flushMemory().catch(() => {})            // 關閉前 flush 待寫的 per-video 記憶 debounce
      await invoke('stop_transcription').catch(() => {})  // 收掉 whisper-server(與 Rust ExitRequested 互保)
      await player.shutdown().catch(() => {})             // 收 mpv（浮動模式 mpv 仍活著，正常 shutdown）
    } finally {
      await win.destroy()
    }
  })
})

onBeforeUnmount(() => { unlistenDrop?.(); unlistenClose?.(); unlistenRec?.() })

// 左鍵按住拖曳超過 4px → 移動視窗(整個 overlay,不限 titlebar)。
// 排除按鈕 / 進度條 / 音量條 / 縮放把手;單擊雙擊不誤觸(雙擊影片仍進全螢幕)。
let dragOrigin: { x: number; y: number } | null = null
function onOverlayPointerDown(e: PointerEvent) {
  if (!normalMode.value) { dragOrigin = null; return }   // 浮動模式：拖曳交給 FloatingCaptions 控件
  if (e.button !== 0) { dragOrigin = null; return }
  const el = e.target as HTMLElement | null
  // 排除：按鈕 / 表單控件(滑桿等) / 進度條 / 音量條 / 縮放把手 / 彈出面板(.sm-pop，如調整面板)
  if (!el || el.closest('button, input, select, textarea, .track, .vol-track, .rh, .sm-pop')) { dragOrigin = null; return }
  dragOrigin = { x: e.clientX, y: e.clientY }
}
function onOverlayPointerMove(e: PointerEvent) {
  if (!dragOrigin) return
  if (!(e.buttons & 1)) { dragOrigin = null; return }   // 左鍵已放開(可能在視窗外)→ 取消,避免殘留誤觸
  if (Math.hypot(e.clientX - dragOrigin.x, e.clientY - dragOrigin.y) > 4) {
    dragOrigin = null
    getCurrentWebviewWindow().startDragging().catch((err) => console.error('[drag] failed', err))
  }
}
function onOverlayPointerUp() { dragOrigin = null }
</script>

<template>
  <div
    class="overlay"
    :class="{ 'cursor-hidden': !visible && normalMode, floating: !normalMode, 'home-solid': normalMode && player.isIdle.value }"
    @dblclick.self="normalMode && player.toggleFullscreen()"
    @pointerdown="onOverlayPointerDown"
    @pointermove="onOverlayPointerMove"
    @pointerup="onOverlayPointerUp"
  >
    <template v-if="normalMode">
      <ResizeHandles />
      <div v-if="showEmptyState" class="empty-state">
        <img :src="wordmarkUrl" class="es-wordmark" alt="Cinelingo" />
        <div class="es-hint">拖曳影片或貼上網址以開始播放</div>
      </div>
      <div v-if="normalMode && player.isIdle.value && audioSource.armed.value" class="viz-center">
        <AudioVisualizer />
      </div>
      <SubtitleOverlay />
      <div v-if="dlText" class="dl-banner">{{ dlText }}</div>
      <Transition name="toast">
        <LoadingOverlay v-if="player.source.resolving || player.source.qualitySwitch.active" />
        <div v-else-if="player.source.loadError" class="url-toast err">{{ player.source.loadError }}</div>
      </Transition>
      <LoadingOverlay v-if="player.state.pausedForCache" :percent="player.state.cacheBufferingState ?? 0" />
      <Transition name="rectoast">
        <button v-if="recordingSaved" class="rec-toast" @click="openRecordingFolder">
          🎙 已儲存錄音 · 點擊開啟資料夾
        </button>
      </Transition>
      <div class="scrim scrim-top" :class="{ hidden: !visible || settings.modal.open }"></div>
      <div class="scrim scrim-bottom" :class="{ hidden: !visible || settings.modal.open }"></div>
      <div
        class="titlebar-wrap"
        :class="{ hidden: !visible || settings.modal.open }"
        @pointerenter="setPointerOverBar(true)"
        @pointerleave="setPointerOverBar(false)"
      >
        <TitleBar />
      </div>
      <div
        class="bar-wrap"
        :class="{ hidden: !visible || settings.modal.open }"
        @pointerenter="setPointerOverBar(true)"
        @pointerleave="setPointerOverBar(false)"
      >
        <ControlBar />
      </div>
      <SettingsModal />
      <EngineProvisionDialog />
    </template>
    <FloatingCaptions v-else />
  </div>
</template>

<style scoped>
.overlay { width: 100vw; height: 100vh; position: relative; }
/* 首頁(idle)沒有影片要透出來 → webview 自己畫不透明底,
   避免透明窗在拖曳/reload 後穿透看到桌面（mpv idle 不重繪）。載入影片後移除此 class 恢復透明。
   上層光暈跟隨主色 var(--accent-rgb)；底層深色漸層為不透明,確保整塊不透明。 */
.overlay.home-solid {
  background:
    radial-gradient(54% 60% at 68% 30%, rgba(var(--accent-rgb), 0.14), transparent 64%),
    radial-gradient(135% 110% at 52% 6%, #16222e 0%, #0a121a 48%, #05080c 100%);
}
.viz-center { position: absolute; inset: 0; display: grid; place-items: center; pointer-events: none; z-index: 2; }
.empty-state {
  position: absolute; inset: 0; z-index: 1; pointer-events: none;
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 18px;
  user-select: none;
}
/* 固定尺寸（不用 max-width %）→ logo 不隨視窗縮放變大變小；拉伸閃爍由手動縮放(ResizeHandles)解決。 */
.empty-state .es-wordmark { width: 340px; height: auto; opacity: 0.95; filter: drop-shadow(0 6px 22px rgba(0,0,0,0.5)); }
.empty-state .es-hint { font-size: 13px; color: #8a8d99; }
.url-toast { position: absolute; top: 50%; left: 50%; transform: translate(-50%,-50%); z-index: 8;
  background: rgba(20,20,24,0.85); color: #e8e8ea; padding: 10px 18px; border-radius: 10px;
  font-size: 13px; pointer-events: none; backdrop-filter: blur(8px); }
.url-toast.err { color: #ff9a9a; }
.rec-toast {
  position: absolute; top: 48px; right: 14px; z-index: 9;
  background: rgba(20,20,24,0.9); color: #e8e8ea; padding: 9px 16px; border-radius: 10px;
  font: 13px var(--font); border: 1px solid rgba(255,255,255,0.14); cursor: pointer;
  backdrop-filter: blur(8px); white-space: nowrap; box-shadow: 0 8px 24px rgba(0,0,0,.4);
}
.rec-toast:hover { background: rgba(36,36,42,0.95); color: #fff; }
/* 右上彈出：由右滑入 + 淡入 */
.rectoast-enter-active, .rectoast-leave-active { transition: opacity .25s ease, transform .25s cubic-bezier(.4,0,.2,1); }
.rectoast-enter-from, .rectoast-leave-to { opacity: 0; transform: translateX(16px); }
.toast-enter-active, .toast-leave-active { transition: opacity 0.4s ease; }
.toast-enter-from, .toast-leave-to { opacity: 0; }
.dl-banner {
  position: absolute; top: 12%; left: 50%; transform: translateX(-50%); z-index: 11;
  background: rgba(0, 0, 0, 0.7); color: #fff; font-size: 13px;
  padding: 6px 14px; border-radius: 8px; pointer-events: none; white-space: nowrap;
}
.scrim {
  position: absolute; left: 0; right: 0; height: 160px;
  pointer-events: none; z-index: 1; transition: opacity 0.4s ease, transform 0.4s ease;
}
.scrim-top.hidden { opacity: 0; transform: translateY(-100%); }
.scrim-bottom.hidden { opacity: 0; transform: translateY(100%); }
.scrim-top {
  top: 0;
  background: linear-gradient(to bottom,
    rgba(0, 0, 0, 0.8) 0%,
    rgba(0, 0, 0, 0.64) 10%,
    rgba(0, 0, 0, 0.45) 24%,
    rgba(0, 0, 0, 0.29) 40%,
    rgba(0, 0, 0, 0.16) 58%,
    rgba(0, 0, 0, 0.07) 78%,
    transparent 100%);
}
.scrim-bottom {
  bottom: 0;
  background: linear-gradient(to top,
    rgba(0, 0, 0, 0.8) 0%,
    rgba(0, 0, 0, 0.64) 10%,
    rgba(0, 0, 0, 0.45) 24%,
    rgba(0, 0, 0, 0.29) 40%,
    rgba(0, 0, 0, 0.16) 58%,
    rgba(0, 0, 0, 0.07) 78%,
    transparent 100%);
}
.titlebar-wrap {
  position: absolute; top: 0; left: 0; right: 0; z-index: 10;
  transition: opacity 0.4s ease, transform 0.4s ease;
}
.titlebar-wrap.hidden { opacity: 0; transform: translateY(-100%); pointer-events: none; }
.bar-wrap {
  position: absolute; left: 0; right: 0; bottom: 0; z-index: 10;
  transition: opacity 0.4s ease, transform 0.4s ease;
}
.bar-wrap.hidden { opacity: 0; transform: translateY(100%); pointer-events: none; }
</style>
