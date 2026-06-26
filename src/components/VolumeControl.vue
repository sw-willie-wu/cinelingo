<script setup lang="ts">
import { ref, computed } from 'vue'
import { usePlayer } from '../player/usePlayer'
import PlayerIcon from './PlayerIcon.vue'

const player = usePlayer()
const trackEl = ref<HTMLElement | null>(null)
const dragging = ref(false)

const vol = computed(() => player.state.volume ?? 0)
const muted = computed(() => player.state.mute === true)
const fillPct = computed(() => `${muted.value ? 0 : vol.value}%`)
const iconName = computed(() => (muted.value || vol.value <= 0 ? 'mute' : 'volume'))

function volFromEvent(e: PointerEvent): number {
  const el = trackEl.value
  if (!el) return 0
  const rect = el.getBoundingClientRect()
  if (rect.width <= 0) return 0
  return Math.min(100, Math.max(0, ((e.clientX - rect.left) / rect.width) * 100))
}

function onPointerDown(e: PointerEvent) {
  dragging.value = true
  ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
  player.setVolume(volFromEvent(e))
}
function onPointerMove(e: PointerEvent) {
  if (dragging.value) player.setVolume(volFromEvent(e))
}
function onPointerUp(e: PointerEvent) {
  if (!dragging.value) return
  dragging.value = false
  ;(e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId)
}
</script>

<template>
  <div class="volume">
    <button class="icon-btn" aria-label="靜音切換" @click="player.toggleMute()">
      <PlayerIcon :name="iconName" :size="20" />
    </button>
    <div
      ref="trackEl"
      class="vol-track"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="onPointerUp"
      @pointercancel="onPointerUp"
    >
      <div class="vol-fill" :style="{ width: fillPct }"></div>
      <div class="vol-thumb" :style="{ left: fillPct }"></div>
    </div>
  </div>
</template>

<style scoped>
.volume { display: flex; align-items: center; gap: 6px; }
.icon-btn {
  background: none; border: none; color: rgba(255, 255, 255, 0.88); cursor: pointer; padding: 4px;
  display: flex; align-items: center; justify-content: center; transition: color 0.18s, filter 0.18s;
}
.icon-btn:hover { color: #fff; filter: drop-shadow(0 0 4px rgba(255, 255, 255, 0.7)) drop-shadow(0 0 11px rgba(255, 255, 255, 0.42)); }
.vol-track {
  position: relative; width: 80px; height: 14px; display: flex; align-items: center;
  cursor: pointer; touch-action: none;
}
.vol-track::before {
  content: ''; position: absolute; left: 0; right: 0; height: 3px;
  background: rgba(255, 255, 255, 0.3); border-radius: 2px;
}
.vol-fill { position: absolute; left: 0; height: 3px; background: #fff; border-radius: 2px; }
.vol-thumb {
  position: absolute; width: 8px; height: 8px; border-radius: 50%; background: #fff;
  transform: translateX(-50%); pointer-events: none;
}
</style>
