<script setup lang="ts">
import { onMounted } from 'vue'
import { useAudioSource } from '../player/useAudioSource'

const a = useAudioSource()
const emit = defineEmits<{ close: [] }>()

onMounted(() => a.refresh())

function pick(sel: Parameters<typeof a.arm>[0]) {
  a.arm(sel)
  emit('close')
}
</script>

<template>
  <div class="src-panel" @click.stop>
    <button class="row" @click="pick({ kind: 'system' })">
      <span class="row-ic">🔊</span>系統輸出（全部聲音）
    </button>
    <template v-if="a.sources.value.processes.length">
      <div class="sec">正在播音的程式</div>
      <button
        v-for="p in a.sources.value.processes"
        :key="p.pid"
        class="row"
        @click="pick({ kind: 'process', name: p.name, pid: p.pid })"
      >
        <span class="row-ic">🖥</span>{{ p.name }}
      </button>
    </template>
    <template v-if="a.sources.value.inputDevices.length">
      <div class="sec">麥克風 / 輸入裝置</div>
      <button
        v-for="d in a.sources.value.inputDevices"
        :key="d.id"
        class="row"
        @click="pick({ kind: 'inputDevice', id: d.id })"
      >
        <span class="row-ic">🎙</span>{{ d.name }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.src-panel {
  position: absolute;
  bottom: calc(100% + 8px);
  right: 0;
  width: 260px;
  background: rgba(24, 25, 30, 0.92);
  backdrop-filter: blur(26px) saturate(1.4);
  -webkit-backdrop-filter: blur(26px) saturate(1.4);
  border: 1px solid rgba(255, 255, 255, 0.12);
  border-radius: 14px;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.6);
  color: #e8e8ea;
  font-size: 13px;
  overflow: hidden;
  z-index: 50;
}
.sec {
  font-size: 10px;
  letter-spacing: 0.5px;
  text-transform: uppercase;
  color: #6b6e7a;
  padding: 9px 14px 4px;
}
.row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 14px;
  background: transparent;
  border: none;
  color: #e8e8ea;
  font-size: 13px;
  text-align: left;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.row:hover {
  background: rgba(255, 255, 255, 0.06);
}
.row-ic {
  font-size: 14px;
  flex-shrink: 0;
}
</style>
