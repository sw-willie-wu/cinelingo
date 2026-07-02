<script setup lang="ts">
import { useAudioSource } from '../player/useAudioSource'
const a = useAudioSource()
</script>
<template>
  <div class="viz">
    <span class="ripple" /><span class="ripple r2" />
    <span class="core" :style="{ transform: `scale(${1 + a.level.value * 0.6})`, opacity: 0.5 + a.level.value * 0.5 }" />
  </div>
</template>
<style scoped>
.viz { position: relative; width: 260px; height: 260px; display: grid; place-items: center; }
.core { width: 64px; height: 64px; border-radius: 50%; background: var(--accent); transition: transform .08s linear, opacity .08s linear; }
.ripple { position: absolute; width: 64px; height: 64px; border-radius: 50%; border: 2px solid var(--accent); opacity: 0; animation: ripple 2.4s ease-out infinite; }
.ripple.r2 { animation-delay: 1.2s; }
@keyframes ripple { 0% { transform: scale(1); opacity: .5 } 100% { transform: scale(3.8); opacity: 0 } }
</style>
