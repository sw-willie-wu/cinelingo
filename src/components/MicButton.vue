<script setup lang="ts">
import { ref } from 'vue'
import { useAudioSource } from '../player/useAudioSource'
import PlayerIcon from './PlayerIcon.vue'
import AudioSourcePanel from './AudioSourcePanel.vue'

defineProps<{ disabled?: boolean }>()

const a = useAudioSource()
const open = ref(false)

function toggle() {
  open.value = !open.value
}
</script>

<template>
  <div class="mic-wrap">
    <button
      class="btn"
      :class="{ armed: a.armed.value }"
      :disabled="disabled"
      aria-label="選擇音源"
      title="選擇音源"
      @click="toggle"
    >
      <PlayerIcon name="mic" />
      <span v-if="a.armed.value" class="breath" />
    </button>
    <AudioSourcePanel v-if="open" @close="open = false" />
  </div>
</template>

<style scoped>
.mic-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.btn {
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 11px;
  border: none;
  background: transparent;
  color: rgba(255, 255, 255, 0.88);
  cursor: pointer;
  padding: 0;
  position: relative;
  transition: color 0.18s, filter 0.18s, transform 0.12s;
}
.btn:hover {
  color: #fff;
  filter: drop-shadow(0 0 4px rgba(255, 255, 255, 0.7)) drop-shadow(0 0 11px rgba(255, 255, 255, 0.42));
}
.btn.armed {
  color: var(--accent);
}
.btn.armed:hover {
  filter: drop-shadow(0 0 4px rgba(var(--accent-rgb), 0.8)) drop-shadow(0 0 12px rgba(var(--accent-rgb), 0.5));
}
.btn:disabled {
  opacity: 0.35;
  cursor: default;
}
.breath {
  position: absolute;
  top: 7px;
  right: 7px;
  width: 7px;
  height: 7px;
  background: var(--accent);
  border-radius: 50%;
  pointer-events: none;
  animation: breath 1.6s ease-in-out infinite;
}
@keyframes breath {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 1; }
}
</style>
