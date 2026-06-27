<script setup lang="ts">
import { onMounted } from 'vue'
import { useAudioSource } from '../../player/useAudioSource'

const emit = defineEmits<{ back: [] }>()
const a = useAudioSource()

onMounted(() => a.refresh())

function pick(sel: Parameters<typeof a.arm>[0]) {
  void a.arm(sel)
}
</script>

<template>
  <div class="back" @click="emit('back')">
    <span class="bk">‹</span>
    <span class="ttl">外部音源</span>
  </div>

  <div v-if="a.armed.value" class="status-row">
    <span class="status-dot" />
    <span class="status-txt">擷取中</span>
    <button class="stop-btn" @click="() => void a.disarm()">停用</button>
  </div>

  <div class="lbl">音源</div>
  <ul class="srcs">
    <li
      class="src"
      :class="{ sel: a.armed.value && a.current.value?.kind === 'system' }"
      @click="pick({ kind: 'system' })"
    >
      <span class="ic">🔊</span>系統輸出（全部聲音）
    </li>
  </ul>

  <template v-if="a.sources.value.processes.length">
    <div class="lbl">正在播音的程式</div>
    <ul class="srcs">
      <li
        v-for="p in a.sources.value.processes"
        :key="p.pid"
        class="src"
        :class="{ sel: a.armed.value && a.current.value?.kind === 'process' && a.current.value.name === p.name }"
        @click="pick({ kind: 'process', name: p.name, pid: p.pid })"
      >
        <span class="ic">🖥</span>{{ p.name }}
      </li>
    </ul>
  </template>

  <template v-if="a.sources.value.inputDevices.length">
    <div class="lbl">麥克風 / 輸入裝置</div>
    <ul class="srcs">
      <li
        v-for="d in a.sources.value.inputDevices"
        :key="d.id"
        class="src"
        :class="{ sel: a.armed.value && a.current.value?.kind === 'inputDevice' && a.current.value.id === d.id }"
        @click="pick({ kind: 'inputDevice', id: d.id })"
      >
        <span class="ic">🎙</span>{{ d.name }}
      </li>
    </ul>
  </template>
</template>

<style scoped>
.back { display: flex; align-items: center; gap: 10px; padding: 12px 14px; border-bottom: 1px solid rgba(255,255,255,0.08); cursor: pointer; }
.bk { font-size: 16px; color: #cfd2db; }
.ttl { color: #fff; font-weight: 600; }

.status-row {
  display: flex; align-items: center; gap: 8px;
  padding: 9px 14px; background: rgba(var(--accent-rgb), 0.1);
  border-bottom: 1px solid rgba(255,255,255,0.06);
}
.status-dot {
  width: 7px; height: 7px; border-radius: 50%; background: var(--accent);
  animation: breath 1.6s ease-in-out infinite; flex-shrink: 0;
}
@keyframes breath { 0%, 100% { opacity: 0.4; } 50% { opacity: 1; } }
.status-txt { flex: 1; font-size: 12px; color: var(--accent); }
.stop-btn {
  padding: 3px 10px; border-radius: 7px; border: 1px solid rgba(var(--accent-rgb), 0.4);
  background: transparent; color: var(--accent); font-size: 12px; cursor: pointer;
}
.stop-btn:hover { background: rgba(var(--accent-rgb), 0.15); }

.lbl { font-size: 10px; letter-spacing: .5px; text-transform: uppercase; color: #7f8290; padding: 11px 14px 5px; }

.srcs { list-style: none; margin: 0; padding: 0 6px; }
.src { display: flex; align-items: center; gap: 10px; padding: 10px 12px; border-radius: 9px; cursor: pointer; border: 1px solid transparent; }
.src:hover { background: rgba(255,255,255,0.06); }
.src.sel { background: rgba(var(--accent-rgb),0.16); border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent) inset; }
.ic { font-size: 14px; flex-shrink: 0; }
</style>
