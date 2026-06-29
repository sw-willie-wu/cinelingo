<script setup lang="ts">
import { computed } from 'vue'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { LogicalSize, LogicalPosition } from '@tauri-apps/api/dpi'
import { usePlayer } from '../player/usePlayer'
import { useWindowControls } from '../player/useWindowControls'
import { RESIZE_HANDLES, type ResizeDir } from '../player/resize'

const win = getCurrentWebviewWindow()
const player = usePlayer()
const { isMaximized } = useWindowControls()

// 最大化或全螢幕時不需縮放
const enabled = computed(() => !isMaximized.value && !player.state.fullscreen)

const MIN_W = 480
const MIN_H = 320

// 手動縮放（取代 startResizeDragging）：OS modal 迴圈會凍結 JS/renderer → WebView2 把上一幀
// 直接點陣拉伸到新尺寸（透明窗上 logo 會「放大一瞬間再跳回」）。自己用 pointermove + setSize/
// setPosition 驅動 → renderer 全程活著、每幀以正確尺寸重繪、不再閃。座標用 screenX/Y(皆 logical)。
interface Gesture {
  dir: ResizeDir
  startX: number; startY: number       // 起始指標（logical 螢幕座標）
  x: number; y: number; w: number; h: number  // 起始視窗幾何（logical）
}
let g: Gesture | null = null
let raf = 0
let target: { w: number; h: number; x: number; y: number; movePos: boolean } | null = null

function flush() {
  raf = 0
  if (!target) return
  const { w, h, x, y, movePos } = target
  void win.setSize(new LogicalSize(w, h))
  if (movePos) void win.setPosition(new LogicalPosition(x, y))
}
function schedule() {
  if (!raf) raf = requestAnimationFrame(flush)
}

async function onDown(e: PointerEvent, dir: ResizeDir) {
  if (!enabled.value || e.button !== 0) return
  e.preventDefault()
  const sx = e.screenX, sy = e.screenY
  const targetEl = e.currentTarget as HTMLElement
  try { targetEl.setPointerCapture(e.pointerId) } catch { /* ignore */ }
  // 取起始幾何（borderless：outer ≈ inner）。await 期間的移動先被忽略（g 尚未設）。
  const scale = await win.scaleFactor()
  const pos = await win.outerPosition()
  const size = await win.innerSize()
  g = {
    dir, startX: sx, startY: sy,
    x: pos.x / scale, y: pos.y / scale,
    w: size.width / scale, h: size.height / scale,
  }
}

function onMove(e: PointerEvent) {
  if (!g) return
  const dx = e.screenX - g.startX
  const dy = e.screenY - g.startY
  let nx = g.x, ny = g.y, nw = g.w, nh = g.h
  if (g.dir.includes('East')) nw = g.w + dx
  if (g.dir.includes('South')) nh = g.h + dy
  if (g.dir.includes('West')) { nw = g.w - dx; nx = g.x + dx }
  if (g.dir.includes('North')) { nh = g.h - dy; ny = g.y + dy }
  // 夾最小尺寸：靠 West/North 邊時，超過最小要把位置補回來，避免對邊跟著縮。
  if (nw < MIN_W) { if (g.dir.includes('West')) nx -= MIN_W - nw; nw = MIN_W }
  if (nh < MIN_H) { if (g.dir.includes('North')) ny -= MIN_H - nh; nh = MIN_H }
  const movePos = g.dir.includes('West') || g.dir.includes('North')
  target = { w: Math.round(nw), h: Math.round(nh), x: Math.round(nx), y: Math.round(ny), movePos }
  schedule()
}

function onUp(e: PointerEvent) {
  if (!g) return
  g = null
  if (raf) { cancelAnimationFrame(raf); raf = 0 }
  if (target) { flush() }   // 落地套最後一次
  target = null
  try { (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId) } catch { /* ignore */ }
}
</script>

<template>
  <div v-if="enabled" class="resize-handles">
    <div
      v-for="h in RESIZE_HANDLES"
      :key="h.key"
      :class="['rh', `rh-${h.key}`]"
      @pointerdown="onDown($event, h.dir)"
      @pointermove="onMove"
      @pointerup="onUp"
      @pointercancel="onUp"
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
