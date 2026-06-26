<script setup lang="ts">
import { computed, ref } from 'vue'
import { useSubtitles, ccMode } from '../player/useSubtitles'
import { usePlayer } from '../player/usePlayer'
import { useAudioSource } from '../player/useAudioSource'
import { useSettings } from '../player/useSettings'
import { langToWhisper } from '../player/langs'
import { startExternalTranscription, stopExternalTranscription } from '../player/backend'
import PlayerIcon from './PlayerIcon.vue'

const subs = useSubtitles()
const player = usePlayer()
const audioSource = useAudioSource()

const active = computed(() => subs.ccActive.value)
const transcribeOn = ref(false)

const mode = computed(() =>
  ccMode(!player.isIdle.value, audioSource.armed.value)
)

async function onCcExternal() {
  const s = useSettings().state
  if (transcribeOn.value) {
    await stopExternalTranscription()
    transcribeOn.value = false
    return
  }
  const { lang, prompt } = langToWhisper(s.liveSubs.sourceLang)
  if (!lang) {
    player.notify('請先在設定→即時字幕將來源語言設為明確語言')
    return
  }
  await startExternalTranscription(
    s.liveSubs.model,
    lang,
    prompt ?? '',
    s.liveSubs.vad.threshold,
    s.liveSubs.vad.minSilenceMs,
  )
  transcribeOn.value = true
}

function handleClick() {
  if (mode.value === 'file') subs.toggleCc()
  else if (mode.value === 'external') onCcExternal()
}
</script>
<template>
  <div class="sub-controls">
    <button
      class="btn"
      :class="{ on: active || transcribeOn }"
      :disabled="mode === 'disabled'"
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
