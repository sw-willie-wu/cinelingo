<script setup lang="ts">
import { computed } from 'vue'
import { useSubtitles, ccMode } from '../player/useSubtitles'
import { usePlayer } from '../player/usePlayer'
import { useAudioSource } from '../player/useAudioSource'
import { useModelDownloads } from '../player/useModelDownloads'
import PlayerIcon from './PlayerIcon.vue'

const subs = useSubtitles()
const player = usePlayer()
const audioSource = useAudioSource()

const active = computed(() => subs.ccActive.value)
const md = useModelDownloads()
const mode = computed(() => ccMode(!player.isIdle.value, audioSource.armed.value))
const disabled = computed(() => mode.value === 'disabled' || (mode.value === 'external' && !md.hasWhisperModel.value))

function handleClick() {
  if (!disabled.value) subs.toggleCc()
}
</script>
<template>
  <div class="sub-controls">
    <button
      class="btn"
      :class="{ on: active }"
      :disabled="disabled"
      aria-label="顯示/隱藏字幕"
      title="顯示/隱藏字幕"
      @click="handleClick()"
    >
      <PlayerIcon name="captions-live" />
    </button>
  </div>
</template>

<style scoped>
.sub-controls { display: flex; align-items: center; gap: 5px; }
.btn { width: 40px; height: 40px; display: flex; align-items: center; justify-content: center; border-radius: 11px; border: none; background: transparent; color: rgba(255,255,255,0.88); cursor: pointer; padding: 0; transition: color 0.18s, filter 0.18s; }
.btn:hover { color: #fff; filter: drop-shadow(0 0 4px rgba(255,255,255,0.7)) drop-shadow(0 0 11px rgba(255,255,255,0.42)); }
.btn.on { color: var(--accent); }
.btn.on:hover { filter: drop-shadow(0 0 4px rgba(var(--accent-rgb),0.8)) drop-shadow(0 0 12px rgba(var(--accent-rgb),0.5)); }
.btn:disabled { opacity: 0.35; cursor: not-allowed; filter: none; }
</style>
