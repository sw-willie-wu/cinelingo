<script setup lang="ts">
import { ref, computed } from 'vue'
import { usePlayer } from '../player/usePlayer'
import { timeToFraction, fractionToTime, bufferedFraction } from '../player/seek'
import { formatClock } from '../player/format'

const player = usePlayer()
const trackEl = ref<HTMLElement | null>(null)
const dragging = ref(false)
const dragFraction = ref(0)
const hoverFraction = ref<number | null>(null)

// 切畫質期間凍結在原位(reload 把 time-pos 歸 0 不顯示)；拖曳中用指標 fraction；否則用 time-pos
const playedFraction = computed(() =>
  player.source.qualitySwitch.active
    ? player.source.qualitySwitch.posFrac
    : dragging.value
      ? dragFraction.value
      : timeToFraction(player.state.timePos, player.state.duration))

// 已緩衝比例：僅遠端來源顯示（本地 mpv 隨需讀取、cache 數字會誤導）。
const bufferedFrac = computed(() =>
  player.source.current?.kind === 'remote'
    ? bufferedFraction(player.state.demuxerCacheTime, player.state.duration)
    : 0)

const hoverText = computed(() =>
  hoverFraction.value == null
    ? ''
    : formatClock(fractionToTime(hoverFraction.value, player.state.duration)))

function fractionFromEvent(e: PointerEvent): number {
  const el = trackEl.value
  if (!el) return 0
  const rect = el.getBoundingClientRect()
  if (rect.width <= 0) return 0
  return Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width))
}

function onPointerDown(e: PointerEvent) {
  if (player.state.duration == null) return
  dragging.value = true
  const f = fractionFromEvent(e)
  dragFraction.value = f
  ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  player.seekTo(fractionToTime(f, player.state.duration))
}

function onPointerMove(e: PointerEvent) {
  hoverFraction.value = fractionFromEvent(e)
  if (dragging.value && player.state.duration != null) {
    dragFraction.value = hoverFraction.value
    player.seekTo(fractionToTime(hoverFraction.value, player.state.duration))
  }
}

function onPointerUp(e: PointerEvent) {
  if (!dragging.value) return
  dragging.value = false
  ;(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId)
}

function onPointerLeave() { hoverFraction.value = null }
</script>

<template>
  <div class="seekbar">
    <div
      ref="trackEl"
      class="track"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
      @pointerleave="onPointerLeave"
    >
      <div class="buffered" :style="{ width: `${bufferedFrac * 100}%` }"></div>
      <div class="fill" :style="{ width: `${playedFraction * 100}%` }"></div>
      <div class="thumb" :style="{ left: `${playedFraction * 100}%` }"></div>
      <div
        v-if="hoverFraction !== null"
        class="hover-tip"
        :style="{ left: `${hoverFraction * 100}%` }"
      >{{ hoverText }}</div>
    </div>
  </div>
</template>

<style scoped>
.seekbar { display: flex; align-items: center; width: 100%; box-sizing: border-box; padding: 0 12px 0 8px; }
.track {
  position: relative; flex: 1; height: 14px; display: flex; align-items: center;
  cursor: pointer; touch-action: none;
}
.track::before {
  content: ''; position: absolute; left: 0; right: 0; height: 4px;
  background: rgba(255, 255, 255, 0.3); border-radius: 2px;
}
.buffered { position: absolute; left: 0; height: 4px; background: rgba(255, 255, 255, 0.78); border-radius: 2px; }
.fill { position: absolute; left: 0; height: 4px; background: var(--accent); border-radius: 2px; }
.thumb {
  position: absolute; width: 12px; height: 12px; border-radius: 50%; background: var(--accent);
  transform: translateX(-50%); top: 1px; pointer-events: none;
}
.hover-tip {
  position: absolute; bottom: 18px; transform: translateX(-50%);
  background: rgba(0, 0, 0, 0.85); color: #fff; padding: 2px 6px; border-radius: 3px;
  font: 11px/1 sans-serif; white-space: nowrap; pointer-events: none;
}
</style>
