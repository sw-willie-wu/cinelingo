<script setup lang="ts">
import { computed } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { usePlayer } from '../player/usePlayer'
import { useWindowControls } from '../player/useWindowControls'
import { RESIZE_HANDLES, type ResizeDir } from '../player/resize'

const win = getCurrentWebviewWindow()
const player = usePlayer()
const { isMaximized } = useWindowControls()

// 最大化或全螢幕時不需縮放
const enabled = computed(() => !isMaximized.value && !player.state.fullscreen)

function onDown(e: PointerEvent, dir: ResizeDir) {
  if (!enabled.value) return
  if (e.button !== 0) return                       // 只左鍵
  win.startResizeDragging(dir).catch((err) => console.error('[resize] failed', err))
}
</script>

<template>
  <div v-if="enabled" class="resize-handles">
    <div
      v-for="h in RESIZE_HANDLES"
      :key="h.key"
      :class="['rh', `rh-${h.key}`]"
      @pointerdown="onDown($event, h.dir)"
    ></div>
  </div>
</template>

<style scoped>
.resize-handles { position: absolute; inset: 0; pointer-events: none; z-index: 30; }
.rh { position: absolute; pointer-events: auto; }
.rh-n { top: 0; left: 10px; right: 184px; height: 4px; cursor: ns-resize; } /* 右側讓開 4 顆控制鈕 */
.rh-s { bottom: 0; left: 10px; right: 10px; height: 4px; cursor: ns-resize; }
.rh-w { left: 0; top: 10px; bottom: 10px; width: 4px; cursor: ew-resize; }
.rh-e { right: 0; top: 10px; bottom: 10px; width: 4px; cursor: ew-resize; }
.rh-nw { top: 0; left: 0; width: 10px; height: 10px; cursor: nwse-resize; }
.rh-ne { top: 0; right: 0; width: 6px; height: 6px; cursor: nesw-resize; } /* 縮小以少壓到關閉鈕 */
.rh-sw { bottom: 0; left: 0; width: 10px; height: 10px; cursor: nesw-resize; }
.rh-se { bottom: 0; right: 0; width: 10px; height: 10px; cursor: nwse-resize; }
</style>
